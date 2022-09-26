use crate::error::LdkLiteError as Error;

use crate::payment_store::{PaymentInfo, PAYMENT_INFO_PERSISTENCE_PREFIX};
use crate::{FilesystemLogger, LdkLiteConfig, NetworkGraph, Scorer};

use lightning::routing::scoring::{ProbabilisticScorer, ProbabilisticScoringParameters};
use lightning::util::ser::{Readable, ReadableArgs};
use lightning_persister::FilesystemPersister;

use rand::{thread_rng, RngCore};

use std::fs;
use std::io::{BufReader, Write};
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;
use std::sync::Arc;

pub(crate) fn read_or_generate_seed_file(config: Arc<LdkLiteConfig>) -> Result<[u8; 32], Error> {
	let keys_seed_path = format!("{}/keys_seed", config.storage_dir_path);
	let keys_seed = if let Ok(seed) = fs::read(keys_seed_path.clone()) {
		assert_eq!(seed.len(), 32);
		let mut key = [0; 32];
		key.copy_from_slice(&seed);
		key
	} else {
		let mut key = [0; 32];
		thread_rng().fill_bytes(&mut key);

		let mut f = fs::File::create(keys_seed_path.clone()).map_err(|e| Error::StdIo(e))?;
		f.write_all(&key).expect("Failed to write node keys seed to disk");
		f.sync_all().expect("Failed to sync node keys seed to disk");
		key
	};

	Ok(keys_seed)
}

pub(crate) fn read_network_graph(
	config: Arc<LdkLiteConfig>, logger: Arc<FilesystemLogger>,
) -> Result<NetworkGraph, Error> {
	let ldk_data_dir = format!("{}/ldk", &config.storage_dir_path.clone());
	let network_graph_path = format!("{}/network_graph", ldk_data_dir.clone());

	if let Ok(file) = fs::File::open(network_graph_path) {
		if let Ok(graph) = NetworkGraph::read(&mut BufReader::new(file), Arc::clone(&logger)) {
			return Ok(graph);
		}
	}

	let genesis_hash =
		bitcoin::blockdata::constants::genesis_block(config.network).header.block_hash();
	Ok(NetworkGraph::new(genesis_hash, logger))
}

pub(crate) fn read_scorer(
	config: Arc<LdkLiteConfig>, network_graph: Arc<NetworkGraph>, logger: Arc<FilesystemLogger>,
) -> Scorer {
	let ldk_data_dir = format!("{}/ldk", &config.storage_dir_path.clone());
	let scorer_path = format!("{}/scorer", ldk_data_dir.clone());

	let params = ProbabilisticScoringParameters::default();
	if let Ok(file) = fs::File::open(scorer_path) {
		let args = (params.clone(), Arc::clone(&network_graph), Arc::clone(&logger));
		if let Ok(scorer) = ProbabilisticScorer::read(&mut BufReader::new(file), args) {
			return scorer;
		}
	}
	ProbabilisticScorer::new(params, network_graph, logger)
}

pub(crate) fn read_payment_info(config: Arc<LdkLiteConfig>) -> Result<Vec<PaymentInfo>, Error> {
	let ldk_data_dir = format!("{}/ldk", &config.storage_dir_path.clone());
	let payment_store_path =
		format!("{}/{}", ldk_data_dir.clone(), PAYMENT_INFO_PERSISTENCE_PREFIX);
	let mut payments = Vec::new();

	for entry in fs::read_dir(payment_store_path)? {
		let entry = entry?;
		if entry.path().is_file() {
			let mut f = fs::File::open(entry.path())?;
			if let Ok(payment_info) = PaymentInfo::read(&mut f) {
				payments.push(payment_info);
			}
		}
	}

	Ok(payments)
}

/// Provides an interface that allows a previously persisted key to be unpersisted.
pub trait KVStoreUnpersister {
	/// Unpersist (i.e., remove) the writeable previously persisted under the provided key.
	/// Returns `true` if the key was present, and `false` otherwise.
	fn unpersist(&self, key: &str) -> std::io::Result<bool>;
}

impl KVStoreUnpersister for FilesystemPersister {
	fn unpersist(&self, key: &str) -> std::io::Result<bool> {
		let mut dest_file = PathBuf::from(self.get_data_dir());
		dest_file.push(key);

		if !dest_file.is_file() {
			return Ok(false);
		}

		fs::remove_file(&dest_file)?;
		let parent_directory = dest_file.parent().unwrap();
		let dir_file = fs::OpenOptions::new().read(true).open(parent_directory)?;
		#[cfg(not(target_os = "windows"))]
		{
			unsafe {
				libc::fsync(dir_file.as_raw_fd());
			}
		}

		if dest_file.is_file() {
			return Err(std::io::Error::new(std::io::ErrorKind::Other, "Unpersisting key failed"));
		}

		return Ok(true);
	}
}
