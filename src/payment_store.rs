use crate::hex_utils;
use crate::io_utils::KVStoreUnpersister;
use crate::Error;

use lightning::ln::{PaymentHash, PaymentPreimage, PaymentSecret};
use lightning::util::persist::KVStorePersister;
use lightning::util::ser::{Readable, Writeable, Writer};

use std::collections::HashMap;
use std::iter::FromIterator;
use std::sync::{Arc, Mutex};

/// Represents a payment.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PaymentInfo {
	/// The payment hash, i.e., the hash of the `preimage`.
	pub hash: PaymentHash,
	/// The pre-image used by the payment.
	pub preimage: Option<PaymentPreimage>,
	/// The secret used by the payment.
	pub secret: Option<PaymentSecret>,
	/// The amount transferred.
	pub amount_msat: Option<u64>,
	/// The direction of the payment.
	pub direction: PaymentDirection,
	/// The status of the payment.
	pub status: PaymentStatus,
}

impl Readable for PaymentInfo {
	fn read<R: lightning::io::Read>(
		reader: &mut R,
	) -> Result<Self, lightning::ln::msgs::DecodeError> {
		let preimage = Readable::read(reader)?;
		let hash = Readable::read(reader)?;
		let secret = Readable::read(reader)?;
		let amount_msat = Readable::read(reader)?;
		let direction = Readable::read(reader)?;
		let status = Readable::read(reader)?;
		Ok(PaymentInfo { preimage, hash, secret, amount_msat, direction, status })
	}
}

impl Writeable for PaymentInfo {
	fn write<W: Writer>(&self, writer: &mut W) -> Result<(), lightning::io::Error> {
		self.preimage.write(writer)?;
		self.hash.write(writer)?;
		self.secret.write(writer)?;
		self.amount_msat.write(writer)?;
		self.direction.write(writer)?;
		self.status.write(writer)?;
		Ok(())
	}
}

/// Represents the direction of a payment.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PaymentDirection {
	/// The payment is inbound.
	Inbound,
	/// The payment is outbound.
	Outbound,
}

impl Readable for PaymentDirection {
	fn read<R: lightning::io::Read>(
		reader: &mut R,
	) -> Result<Self, lightning::ln::msgs::DecodeError> {
		let dir: u8 = Readable::read(reader)?;
		match dir {
			0 => Ok(Self::Inbound),
			1 => Ok(Self::Outbound),
			_ => Err(lightning::ln::msgs::DecodeError::InvalidValue),
		}
	}
}

impl Writeable for PaymentDirection {
	fn write<W: Writer>(&self, writer: &mut W) -> Result<(), lightning::io::Error> {
		match self {
			Self::Inbound => (0 as u8).write(writer)?,
			Self::Outbound => (1 as u8).write(writer)?,
		}
		Ok(())
	}
}

/// Represents the current status of a payment.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PaymentStatus {
	/// The payment is still pending.
	Pending,
	/// The payment suceeded.
	Succeeded,
	/// The payment failed.
	Failed,
}

impl Readable for PaymentStatus {
	fn read<R: lightning::io::Read>(
		reader: &mut R,
	) -> Result<Self, lightning::ln::msgs::DecodeError> {
		let status: u8 = Readable::read(reader)?;
		match status {
			0 => Ok(Self::Pending),
			1 => Ok(Self::Succeeded),
			2 => Ok(Self::Failed),
			_ => Err(lightning::ln::msgs::DecodeError::InvalidValue),
		}
	}
}

impl Writeable for PaymentStatus {
	fn write<W: Writer>(&self, writer: &mut W) -> Result<(), lightning::io::Error> {
		match self {
			Self::Pending => (0 as u8).write(writer)?,
			Self::Succeeded => (1 as u8).write(writer)?,
			Self::Failed => (2 as u8).write(writer)?,
		}
		Ok(())
	}
}

/// The payment information will be persisted under this prefix.
pub(crate) const PAYMENT_INFO_PERSISTENCE_PREFIX: &str = "payments";

pub(crate) struct PaymentInfoStorage<K: KVStorePersister + KVStoreUnpersister> {
	payments: Mutex<HashMap<PaymentHash, PaymentInfo>>,
	persister: Arc<K>,
}

impl<K: KVStorePersister + KVStoreUnpersister> PaymentInfoStorage<K> {
	pub(crate) fn new(persister: Arc<K>) -> Self {
		let payments = Mutex::new(HashMap::new());
		Self { payments, persister }
	}

	pub(crate) fn from_payments(mut payments: Vec<PaymentInfo>, persister: Arc<K>) -> Self {
		let payments = Mutex::new(HashMap::from_iter(
			payments.drain(..).map(|payment_info| (payment_info.hash, payment_info)),
		));
		Self { payments, persister }
	}

	pub(crate) fn insert_payment(&self, payment_info: PaymentInfo) -> Result<(), Error> {
		let mut locked_payments = self.payments.lock().unwrap();

		let payment_hash = payment_info.hash.clone();
		locked_payments.insert(payment_hash.clone(), payment_info.clone());

		let key = format!(
			"{}/{}",
			PAYMENT_INFO_PERSISTENCE_PREFIX,
			hex_utils::to_string(&payment_hash.0)
		);
		self.persister.persist(&key, &payment_info)?;

		return Ok(());
	}

	pub(crate) fn remove_payment(&self, payment_hash: &PaymentHash) -> Result<(), Error> {
		let key = format!(
			"{}/{}",
			PAYMENT_INFO_PERSISTENCE_PREFIX,
			hex_utils::to_string(&payment_hash.0)
		);
		self.persister.unpersist(&key)?;
		Ok(())
	}

	pub(crate) fn payment(&self, payment_hash: &PaymentHash) -> Option<PaymentInfo> {
		self.payments.lock().unwrap().get(payment_hash).cloned()
	}

	pub(crate) fn contains_payment(&self, payment_hash: &PaymentHash) -> bool {
		self.payments.lock().unwrap().contains_key(payment_hash)
	}

	pub(crate) fn set_payment_status(
		&self, payment_hash: &PaymentHash, payment_status: PaymentStatus,
	) -> Result<(), Error> {
		let mut locked_payments = self.payments.lock().unwrap();

		if let Some(p) = locked_payments.get_mut(payment_hash) {
			p.status = payment_status;

			let key = format!(
				"{}/{}",
				PAYMENT_INFO_PERSISTENCE_PREFIX,
				hex_utils::to_string(&payment_hash.0)
			);
			self.persister.persist(&key, p)?;
		}
		Ok(())
	}
}
