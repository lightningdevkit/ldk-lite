use crate::error::LdkLiteError as Error;

use crate::{LdkLiteConfig, NetworkGraph, Scorer, FilesystemLogger};
use crate::hex;

use lightning::routing::scoring::{ProbabilisticScorer, ProbabilisticScoringParameters};
use lightning::util::ser::ReadableArgs;

use bitcoin::secp256k1::PublicKey;

use rand::{thread_rng, RngCore};

use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::net::{SocketAddr, ToSocketAddrs};
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

pub(crate) fn read_channel_peer_data(
	config: Arc<LdkLiteConfig>,
) -> Result<HashMap<PublicKey, SocketAddr>, Error> {
	let ldk_data_dir = format!("{}/ldk", &config.storage_dir_path.clone());
	let peer_data_path = format!("{}/channel_peer_data", ldk_data_dir.clone());
	let mut peer_data = HashMap::new();

	if let Ok(file) = fs::File::open(peer_data_path) {
		let reader = BufReader::new(file);
		for line in reader.lines() {
			match parse_peer_info(line.unwrap()) {
				Ok((pubkey, socket_addr)) => {
					peer_data.insert(pubkey, socket_addr);
				}
				Err(e) => return Err(e),
			}
		}
	}
	Ok(peer_data)
}

pub(crate) fn persist_channel_peer(
	config: Arc<LdkLiteConfig>, peer_info: &str,
) -> std::io::Result<()> {
	let ldk_data_dir = format!("{}/ldk", &config.storage_dir_path.clone());
	let peer_data_path = format!("{}/channel_peer_data", ldk_data_dir.clone());
	let mut file = fs::OpenOptions::new().create(true).append(true).open(peer_data_path)?;
	file.write_all(format!("{}\n", peer_info).as_bytes())
}


// TODO: handle different kinds of NetAddress, e.g., the Hostname field.
pub(crate) fn parse_peer_info(
	peer_pubkey_and_ip_addr: String,
) -> Result<(PublicKey, SocketAddr), Error> {
	let mut pubkey_and_addr = peer_pubkey_and_ip_addr.split("@");
	let pubkey = pubkey_and_addr.next();
	let peer_addr_str = pubkey_and_addr.next();
	if peer_addr_str.is_none() || peer_addr_str.is_none() {
		return Err(Error::PeerInfoParse(
			"Incorrect format. Should be formatted as: `pubkey@host:port`.",
		));
	}

	let peer_addr = peer_addr_str.unwrap().to_socket_addrs().map(|mut r| r.next());
	if peer_addr.is_err() || peer_addr.as_ref().unwrap().is_none() {
		return Err(Error::PeerInfoParse("Couldn't parse pubkey@host:port into a socket address."));
	}

	let pubkey = hex::to_compressed_pubkey(pubkey.unwrap());
	if pubkey.is_none() {
		return Err(Error::PeerInfoParse("Unable to parse pubkey for node."));
	}

	Ok((pubkey.unwrap(), peer_addr.unwrap().unwrap()))
}