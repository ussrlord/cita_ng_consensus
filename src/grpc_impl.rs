use cita_ng_proto::consensus_grpc::{ConsensusService, ConsensusServiceServer};
use cita_ng_proto::network;
use cita_ng_proto::{common, consensus};
use futures::future::ok;
use grpc::ServerBuilder;
use log::info;

use crate::ConsensusMsg;
use cita_ng_proto::common::CompactBlock;
use crossbeam_channel::Sender;
use protobuf::parse_from_bytes;
use std::thread;

struct ConsensusServerImpl {
    s: Sender<ConsensusMsg>,
}

impl ConsensusService for ConsensusServerImpl {
    fn proc_network_msg(
        &self,
        _o: ::grpc::RequestOptions,
        p: network::NetworkMsg,
    ) -> ::grpc::SingleResponse<common::SimpleResponse> {
        if let Ok(blk) = parse_from_bytes::<CompactBlock>(&p.msg) {
            let _ = self.s.send(ConsensusMsg::BLOCK(blk));
            let mut ret = common::SimpleResponse::new();
            ret.set_is_success(true);
            grpc::SingleResponse::no_metadata(ok(ret))
        } else {
            let mut ret = common::SimpleResponse::new();
            ret.set_is_success(false);
            grpc::SingleResponse::no_metadata(ok(ret))
        }
    }

    fn reconfigure(
        &self,
        _o: ::grpc::RequestOptions,
        p: consensus::ConsensusConfiguration,
    ) -> ::grpc::SingleResponse<common::SimpleResponse> {
        let _ = self.s.send(ConsensusMsg::CONFIG(p));
        let mut ret = common::SimpleResponse::new();
        ret.set_is_success(true);
        grpc::SingleResponse::no_metadata(ok(ret))
    }
}

pub fn init_grpc_server(s: Sender<ConsensusMsg>, port: u16) {
    let mut server = ServerBuilder::new_plain();
    let addr = format!("0.0.0.0:{}", port);
    info!("grpc server address: {}", addr);
    server.http.set_addr(addr).unwrap();
    server.add_service(ConsensusServiceServer::new_service_def(
        ConsensusServerImpl { s },
    ));
    let _server = server.build().expect("server");

    // This function should not exit
    // Or grpc server will exit too
    loop {
        thread::park();
    }
}
