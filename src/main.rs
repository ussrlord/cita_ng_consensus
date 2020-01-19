mod config;
mod grpc_impl;

use blake2b_simd::blake2b;
use cita_ng_proto::common::CompactBlock;
use cita_ng_proto::consensus::ConsensusConfiguration;
use cita_ng_proto::network::NetworkMsg;
use cita_ng_proto::network_grpc::NetworkService;
use cita_ng_proto::network_grpc::NetworkServiceClient;
use cita_ng_proto::utxo::GetProposalRequest;
use cita_ng_proto::utxo_grpc::UtoxService;
use cita_ng_proto::utxo_grpc::UtoxServiceClient;
use clap::App;
use config::ConsensusConfig;
use crossbeam_channel::{select, tick, unbounded};
use grpc::ClientStubExt;
use grpc_impl::init_grpc_server;
use log::{debug, error};
use proof_of_sleep::POS;
use protobuf::Message;
use std::convert::TryInto;
use std::thread;
use std::time::Duration;

pub enum ConsensusMsg {
    BLOCK(CompactBlock),
    CONFIG(ConsensusConfiguration),
}

fn main() {
    env_logger::init();

    // init app
    let matches = App::new("cita_ng_consensus")
        .version("0.0.1")
        .author("Rivtower")
        .about("CITA NG Block Chain Node powered by Rust")
        .args_from_usage("-c, --config=[FILE] 'Sets a custom config file'")
        .get_matches();

    // set default values of config file
    let config_file = matches.value_of("config").unwrap_or("consensus.toml");
    debug!("Consensus config is {:?}", config_file);

    // parse config file
    let config = ConsensusConfig::new(&config_file);

    let (tx, rx) = unbounded();

    // start grpc server
    let grpc_server_port = config.grpc_server_port.unwrap_or(5000);
    thread::spawn(move || {
        init_grpc_server(tx.clone(), grpc_server_port);
    });

    // start grpc client
    let utxo_grpc_port = config.utxo_grpc_port.unwrap_or(5000);
    let utxo_grpc_client = UtoxServiceClient::new_plain("utxo", utxo_grpc_port, Default::default())
        .expect("client init failed");
    let network_grpc_port = config.network_grpc_port.unwrap_or(5000);
    let network_grpc_client =
        NetworkServiceClient::new_plain("network", network_grpc_port, Default::default())
            .expect("client init failed");

    // init consensus
    let block_interval = 3;
    let target = 4;
    let ticker = tick(Duration::from_secs(block_interval));

    loop {
        select! {
            // deal message from grpc
            recv(rx) -> msg => {
                match msg {
                    Ok(data) => {
                        match data {
                            ConsensusMsg::BLOCK(blk) => {
                                let mut blk_clone = blk.clone();
                                let proof = blk_clone.mut_header().take_proof();
                                if proof.len() == std::mem::size_of::<u64>() {
                                    let nonce = u64::from_be_bytes(proof.as_slice().try_into().unwrap());
                                    let blk_bytes = blk_clone.write_to_bytes().unwrap();
                                    let blk_hash = blake2b(&blk_bytes);
                                    let pos = POS::new(target as usize, 1);
                                    if pos.check_nonce(blk_hash.as_bytes(), nonce) {
                                        let _ = utxo_grpc_client.verify_proposal_blocks(grpc::RequestOptions::new(), blk).wait_drop_metadata();
                                    }
                                }
                            },
                            ConsensusMsg::CONFIG(_config) => {

                            }
                        }

                    },
                    Err(err) => error!("Receive data error {:?}", err),
                }
            }
            recv(ticker) -> _ => {
                let mut proposal_req = GetProposalRequest::new();
                proposal_req.set_height(0);
                let mut proposal = utxo_grpc_client.get_proposal_block(grpc::RequestOptions::new(), proposal_req).wait_drop_metadata().unwrap();
                let proposal_bytes = proposal.clone().write_to_bytes().unwrap();
                let block_hash = blake2b(&proposal_bytes);
                let pos = POS::new(target as usize, 1);
                if let Some(nonce) = pos.mint(block_hash.as_bytes()) {
                    proposal.mut_header().set_proof(nonce.to_be_bytes()[0..].to_vec());
                    let _ = utxo_grpc_client.verify_proposal_blocks(grpc::RequestOptions::new(), proposal.clone()).wait_drop_metadata();
                    let mut msg = NetworkMsg::new();
                    msg.set_module("consensus".to_owned());
                    msg.set_field_type("proposal".to_owned());
                    msg.set_msg(proposal.write_to_bytes().unwrap());
                    let _ = network_grpc_client.broadcast(grpc::RequestOptions::new(), msg).wait_drop_metadata();
                }
            }
        }
    }
}
