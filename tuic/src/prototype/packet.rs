use super::side::{self, Side, SideMarker};
use crate::protocol::{Address, Header, Packet as PacketHeader};

pub struct Packet<M>
where
    M: SideMarker,
{
    inner: Side<Tx, Rx>,
    _marker: M,
}

pub struct Tx {
    assoc_id: u16,
    pkt_id: u16,
    addr: Address,
    max_pkt_size: usize,
}

pub struct Rx;

impl Packet<side::Tx> {
    pub(super) fn new(assoc_id: u16, pkt_id: u16, addr: Address, max_pkt_size: usize) -> Self {
        Self {
            inner: Side::Tx(Tx {
                assoc_id,
                pkt_id,
                addr,
                max_pkt_size,
            }),
            _marker: side::Tx,
        }
    }

    pub fn into_fragments<'a>(self, payload: &'a [u8]) -> Fragment<'a> {
        let Side::Tx(tx) = self.inner else { unreachable!() };
        Fragment::new(tx.assoc_id, tx.pkt_id, tx.addr, tx.max_pkt_size, payload)
    }
}

pub struct Fragment<'a> {
    assoc_id: u16,
    pkt_id: u16,
    addr: Address,
    max_pkt_size: usize,
    frag_total: u8,
    next_frag_id: u8,
    next_frag_start: usize,
    payload: &'a [u8],
}

impl<'a> Fragment<'a> {
    fn new(
        assoc_id: u16,
        pkt_id: u16,
        addr: Address,
        max_pkt_size: usize,
        payload: &'a [u8],
    ) -> Self {
        let first_frag_size = max_pkt_size - PacketHeader::len_without_addr() - addr.len();
        let frag_size_addr_none =
            max_pkt_size - PacketHeader::len_without_addr() - Address::None.len();

        let frag_total = if first_frag_size < payload.len() {
            (1 + (payload.len() - first_frag_size) / frag_size_addr_none + 1) as u8
        } else {
            1u8
        };

        Self {
            assoc_id,
            pkt_id,
            addr,
            max_pkt_size,
            frag_total,
            next_frag_id: 0,
            next_frag_start: 0,
            payload,
        }
    }
}

impl<'a> Iterator for Fragment<'a> {
    type Item = (Header, &'a [u8]);

    fn next(&mut self) -> Option<Self::Item> {
        if self.next_frag_id < self.frag_total {
            let payload_size =
                self.max_pkt_size - PacketHeader::len_without_addr() - self.addr.len();
            let next_frag_end = (self.next_frag_start + payload_size).min(self.payload.len());

            let header = Header::Packet(PacketHeader::new(
                self.assoc_id,
                self.pkt_id,
                self.frag_total,
                self.next_frag_id,
                (next_frag_end - self.next_frag_start) as u16,
                self.addr.take(),
            ));

            let payload = &self.payload[self.next_frag_start..next_frag_end];

            self.next_frag_id += 1;
            self.next_frag_start = next_frag_end;

            Some((header, payload))
        } else {
            None
        }
    }
}

impl ExactSizeIterator for Fragment<'_> {
    fn len(&self) -> usize {
        self.frag_total as usize
    }
}