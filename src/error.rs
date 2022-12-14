use bdk::blockchain::esplora;
use lightning::ln::msgs;
use lightning::util::errors;
use lightning_invoice::payment;
use std::fmt;
use std::io;
use std::time;

#[derive(Debug)]
/// An error that possibly needs to be handled by the user.
pub enum LdkLiteError {
	/// Returned when trying to start LdkLite while it is already running.
	AlreadyRunning,
	/// Returned when trying to stop LdkLite while it is not running.
	NotRunning,
	/// The funding transaction could not be created.
	FundingTxCreationFailed,
	/// A network connection has been closed.
	ConnectionFailed,
	/// Payment of the given invoice has already been intiated.
	NonUniquePaymentHash,
	/// A given peer info could not be parsed.
	PeerInfoParse(&'static str),
	/// A wrapped LDK `APIError`
	LdkApi(errors::APIError),
	/// A wrapped LDK `DecodeError`
	LdkDecode(msgs::DecodeError),
	/// A wrapped LDK `PaymentError`
	LdkPayment(payment::PaymentError),
	/// A wrapped LDK `SignOrCreationError`
	LdkInvoiceCreation(lightning_invoice::SignOrCreationError),
	/// A wrapped BDK error
	Bdk(bdk::Error),
	/// A wrapped `EsploraError`
	Esplora(esplora::EsploraError),
	/// A wrapped `Bip32` error
	Bip32(bitcoin::util::bip32::Error),
	/// A wrapped `std::io::Error`
	StdIo(io::Error),
	/// A wrapped `SystemTimeError`
	StdTime(time::SystemTimeError),
}

impl fmt::Display for LdkLiteError {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match *self {
			LdkLiteError::AlreadyRunning => write!(f, "LDKLite is already running."),
			LdkLiteError::NotRunning => write!(f, "LDKLite is not running."),
			LdkLiteError::FundingTxCreationFailed => {
				write!(f, "the funding transaction could not be created")
			}
			LdkLiteError::ConnectionFailed => write!(f, "network connection closed"),
			LdkLiteError::NonUniquePaymentHash => write!(f, "an invoice must not get payed twice."),
			LdkLiteError::PeerInfoParse(ref e) => {
				write!(f, "given peer info could not be parsed: {}", e)
			}
			LdkLiteError::LdkDecode(ref e) => write!(f, "LDK decode error: {}", e),
			LdkLiteError::LdkApi(ref e) => write!(f, "LDK API error: {:?}", e),
			LdkLiteError::LdkPayment(ref e) => write!(f, "LDK payment error: {:?}", e),
			LdkLiteError::LdkInvoiceCreation(ref e) => {
				write!(f, "LDK invoice sign or creation error: {:?}", e)
			}
			LdkLiteError::Bdk(ref e) => write!(f, "BDK error: {}", e),
			LdkLiteError::Esplora(ref e) => write!(f, "Esplora error: {}", e),
			LdkLiteError::Bip32(ref e) => write!(f, "Bitcoin error: {}", e),
			LdkLiteError::StdIo(ref e) => write!(f, "IO error: {}", e),
			LdkLiteError::StdTime(ref e) => write!(f, "time error: {}", e),
		}
	}
}

impl From<errors::APIError> for LdkLiteError {
	fn from(e: errors::APIError) -> Self {
		Self::LdkApi(e)
	}
}

impl From<msgs::DecodeError> for LdkLiteError {
	fn from(e: msgs::DecodeError) -> Self {
		Self::LdkDecode(e)
	}
}

impl From<payment::PaymentError> for LdkLiteError {
	fn from(e: payment::PaymentError) -> Self {
		Self::LdkPayment(e)
	}
}

impl From<lightning_invoice::SignOrCreationError> for LdkLiteError {
	fn from(e: lightning_invoice::SignOrCreationError) -> Self {
		Self::LdkInvoiceCreation(e)
	}
}

impl From<bdk::Error> for LdkLiteError {
	fn from(e: bdk::Error) -> Self {
		Self::Bdk(e)
	}
}

impl From<bdk::sled::Error> for LdkLiteError {
	fn from(e: bdk::sled::Error) -> Self {
		Self::Bdk(bdk::Error::Sled(e))
	}
}

impl From<bitcoin::util::bip32::Error> for LdkLiteError {
	fn from(e: bitcoin::util::bip32::Error) -> Self {
		Self::Bip32(e)
	}
}

impl From<io::Error> for LdkLiteError {
	fn from(e: io::Error) -> Self {
		Self::StdIo(e)
	}
}

impl From<time::SystemTimeError> for LdkLiteError {
	fn from(e: time::SystemTimeError) -> Self {
		Self::StdTime(e)
	}
}

impl From<esplora::EsploraError> for LdkLiteError {
	fn from(e: esplora::EsploraError) -> Self {
		Self::Esplora(e)
	}
}
