use super::consts::*;
use crate::{
    common::{
        enums::{IpProtocol, PacketDirection},
        protocol_logs::{DnsInfo, LogMessageType},
        IPV4_ADDR_LEN, IPV6_ADDR_LEN,
    },
    error::{Error, Result},
    utils::{bytes::read_u16_be, net::parse_ip_slice},
};

#[derive(Debug, Default)]
struct DnsLog {
    info: DnsInfo,

    pub domain_type: u16,

    msg_type: LogMessageType,
}

impl DnsLog {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn reset_logs(&mut self) {
        self.info.trans_id = 0;
        self.info.query_type = 0;
        self.info.query_name = String::new();
        self.info.answers = String::new();
    }

    fn get_log_data_special_info(self) {
        self.info;
    }

    pub fn decode_name(&self, payload: &[u8], g_offset: usize) -> Result<(String, usize)> {
        let mut l_offset = g_offset;
        let mut index = g_offset;
        let mut buffer = String::new();

        if payload.len() <= l_offset {
            let err_msg = format!("payload too short: {}", payload.len());
            return Err(Error::DnsLogParse(err_msg));
        }

        if payload[index] == DNS_NAME_TAIL {
            return Ok((buffer, index + 1));
        }

        while payload[index] != DNS_NAME_TAIL {
            if payload[index] == DNS_NAME_RESERVERD_40 || payload[index] == DNS_NAME_RESERVERD_80 {
                let err_msg = format!("dns name label type error: {}", payload[index]);
                return Err(Error::DnsLogParse(err_msg));
            } else if payload[index] == DNS_NAME_COMPRESS_POINTER {
                if index + 2 > payload.len() {
                    let err_msg = format!("dns name invalid index: {}", index - 2);
                    return Err(Error::DnsLogParse(err_msg));
                }
                let index_ptr = read_u16_be(&payload[index..]) as usize & 0x3fff;
                if index_ptr > payload.len() {
                    let err_msg = format!("dns name offset too large: {}", index_ptr);
                    return Err(Error::DnsLogParse(err_msg));
                }
                index = index_ptr;
            } else {
                let size = index + 1 + payload[index] as usize;
                if size > payload.len()
                    || (size > g_offset && (size - g_offset) > DNS_NAME_MAX_SIZE)
                {
                    let err_msg = format!("dns name invalid index: {}", size);
                    return Err(Error::DnsLogParse(err_msg));
                }

                if buffer.len() > 0 {
                    buffer.push('.');
                }
                match std::str::from_utf8(&payload[index + 1..size]) {
                    Ok(s) => {
                        buffer.push_str(s);
                    }
                    Err(e) => {
                        let err_msg = format!("decode name error {}", e);
                        return Err(Error::DnsLogParse(err_msg));
                    }
                }
                index = size;
                if index >= payload.len() {
                    let err_msg = format!("dns name invalid index: {}", index);
                    return Err(Error::DnsLogParse(err_msg));
                }

                if index > l_offset {
                    l_offset = size;
                } else if payload[index] == DNS_NAME_TAIL {
                    l_offset += l_offset;
                }
            }
        }

        Ok((buffer, l_offset + 1))
    }

    pub fn decode_question(&mut self, payload: &[u8], g_offset: usize) -> Result<usize> {
        let (name, offset) = self.decode_name(payload, g_offset)?;
        let qtype_size = payload[offset..].len();
        if qtype_size < QUESTION_CLASS_TYPE_SIZE {
            let err_msg = format!("question length error: {}", qtype_size);
            return Err(Error::DnsLogParse(err_msg));
        }

        if self.info.query_name.len() > 0 {
            self.info.query_name.push(DOMAIN_NAME_SPLIT);
        }
        self.info.query_name.push_str(&name);
        if self.info.query_type == DNS_REQUEST {
            self.domain_type = read_u16_be(&payload[offset..]);
            self.msg_type = LogMessageType::Request;
        }

        Ok(offset + QUESTION_CLASS_TYPE_SIZE)
    }

    pub fn decode_resource_record(&mut self, payload: &[u8], g_offset: usize) -> Result<usize> {
        let (_, offset) = self.decode_name(payload, g_offset)?;

        if offset > payload.len() {
            let err_msg = format!("payload length error: {}", payload.len());
            return Err(Error::DnsLogParse(err_msg));
        }

        let resource_len = payload[offset..].len();
        if resource_len < RR_RDATA_OFFSET {
            let err_msg = format!("resource record length error: {}", resource_len);
            return Err(Error::DnsLogParse(err_msg));
        }

        self.domain_type = read_u16_be(&payload[offset..]);
        let data_length = read_u16_be(&payload[offset + RR_DATALENGTH_OFFSET..]) as usize;
        if data_length != 0 {
            self.decode_rdata(payload, offset + RR_RDATA_OFFSET, data_length)?;
        }

        Ok(offset + RR_RDATA_OFFSET + data_length)
    }

    pub fn decode_rdata(
        &mut self,
        payload: &[u8],
        g_offset: usize,
        data_length: usize,
    ) -> Result<()> {
        let answer_name_len = self.info.answers.len();
        if answer_name_len > 0
            && self.info.answers[answer_name_len - 1..] != DOMAIN_NAME_SPLIT.to_string()
        {
            self.info.answers.push(DOMAIN_NAME_SPLIT);
        }

        match self.domain_type {
            DNS_TYPE_A | DNS_TYPE_AAAA => match data_length {
                IPV4_ADDR_LEN | IPV6_ADDR_LEN => {
                    if let Some(ipaddr) = parse_ip_slice(payload) {
                        self.info.answers.push_str(&ipaddr.to_string());
                    }
                }
                _ => {
                    let err_msg = format!("{} type invalid data {}", self.domain_type, data_length);
                    return Err(Error::DnsLogParse(err_msg));
                }
            },
            DNS_TYPE_NS | DNS_TYPE_DNAME => {
                if data_length > DNS_NAME_MAX_SIZE {
                    let err_msg = format!("{} type invalid data {}", self.domain_type, data_length);
                    return Err(Error::DnsLogParse(err_msg));
                }

                let (name, _) = self.decode_name(payload, g_offset)?;
                self.info.answers.push_str(&name);
            }
            DNS_TYPE_WKS => {
                if data_length < DNS_TYPE_WKS_LENGTH {
                    let err_msg = format!("{} type invalid data {}", self.domain_type, data_length);
                    return Err(Error::DnsLogParse(err_msg));
                }
                if let Some(ipaddr) = parse_ip_slice(payload) {
                    self.info.answers.push_str(&ipaddr.to_string());
                }
            }
            DNS_TYPE_PTR => {
                if data_length != DNS_TYPE_PTR_LENGTH {
                    let err_msg = format!("{} type invalid data {}", self.domain_type, data_length);
                    return Err(Error::DnsLogParse(err_msg));
                }
            }
            _ => {
                let err_msg = format!("{} type invalid data {}", self.domain_type, data_length);
                return Err(Error::DnsLogParse(err_msg));
            }
        }
        Ok(())
    }

    pub fn decode_payload(&mut self, payload: &[u8]) -> Result<()> {
        if payload.len() <= DNS_HEADER_SIZE {
            let err_msg = format!("dns payload length too short:{}", payload.len());
            return Err(Error::DnsLogParse(err_msg));
        }
        self.info.trans_id = read_u16_be(&payload[..DNS_HEADER_FLAGS_OFFSET]);
        self.info.query_type = payload[DNS_HEADER_FLAGS_OFFSET] & 0x80;

        let qd_count = read_u16_be(&payload[DNS_HEADER_QDCOUNT_OFFSET..]);
        let an_count = read_u16_be(&payload[DNS_HEADER_ANCOUNT_OFFSET..]);
        let ns_count = read_u16_be(&payload[DNS_HEADER_NSCOUNT_OFFSET..]);

        let mut g_offset = DNS_HEADER_SIZE;

        for _i in 0..qd_count {
            g_offset = self.decode_question(payload, g_offset)?;
        }

        if self.info.query_type == DNS_RESPONSE {
            self.info.query_type = 1;
            for _i in 0..an_count {
                g_offset = self.decode_resource_record(payload, g_offset)?;
            }

            for _i in 0..ns_count {
                g_offset = self.decode_resource_record(payload, g_offset)?;
            }

            self.msg_type = LogMessageType::Response;
        }
        Ok(())
    }

    pub fn parse(
        &mut self,
        payload: &[u8],
        proto: IpProtocol,
        _direction: PacketDirection,
    ) -> Result<()> {
        self.reset_logs();
        match proto {
            IpProtocol::Udp => self.decode_payload(payload),
            IpProtocol::Tcp => {
                if payload.len() <= DNS_TCP_PAYLOAD_OFFSET {
                    let err_msg = format!("dns payload length error:{}", payload.len());
                    return Err(Error::DnsLogParse(err_msg));
                }
                let size = read_u16_be(payload);
                if (size as usize) < payload[DNS_TCP_PAYLOAD_OFFSET..].len() {
                    let err_msg = format!("dns payload length error:{}", size);
                    return Err(Error::DnsLogParse(err_msg));
                }
                self.decode_payload(&payload[DNS_TCP_PAYLOAD_OFFSET..])
            }
            _ => {
                let err_msg = format!("dns payload length error:{}", payload.len());
                return Err(Error::DnsLogParse(err_msg));
            }
        }
    }
}

#[cfg(test)]
mod test {
    use std::fs;
    use std::path::Path;

    use super::*;

    use crate::{common::enums::PacketDirection, utils::test::Capture};

    const FILE_DIR: &str = "resources/test/flow_generator/dns";

    fn run(name: &str) -> String {
        let capture = Capture::load_pcap(Path::new(FILE_DIR).join(name), None);
        let mut packets = capture.as_meta_packets();
        if packets.is_empty() {
            return "".to_string();
        }

        let mut output = String::new();
        let first_dst_port = packets[0].lookup_key.dst_port;
        for packet in packets.iter_mut() {
            packet.direction = if packet.lookup_key.dst_port == first_dst_port {
                PacketDirection::ClientToServer
            } else {
                PacketDirection::ServerToClient
            };
            let payload = match packet.get_l4_payload() {
                Some(p) => p,
                None => continue,
            };

            let mut dns = DnsLog::default();
            let _ = dns.parse(payload, packet.lookup_key.proto, packet.direction);
            output.push_str(&format!("{:?}\r\n", dns.info));
        }
        output
    }

    #[test]
    fn check() {
        let files = vec![("dns.pcap", "dns.result")];

        for item in files.iter() {
            let expected = fs::read_to_string(&Path::new(FILE_DIR).join(item.1)).unwrap();
            let output = run(item.0);

            if output != expected {
                let output_path = Path::new("actual.txt");
                fs::write(&output_path, &output).unwrap();
                assert!(
                    output == expected,
                    "output different from expected {}, written to {:?}",
                    item.1,
                    output_path
                );
            }
        }
    }
}
