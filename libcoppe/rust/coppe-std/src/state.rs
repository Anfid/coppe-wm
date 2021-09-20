pub mod clients {
    use coppe_common::client::Client;
    use coppe_common::encoding::{Decode, DecodeError};
    use coppe_core::ffi;

    pub fn read() -> Result<Vec<Client>, DecodeError> {
        let len = ffi::clients_len();
        let mut buffer = vec![0; len];
        ffi::clients_read(buffer.as_mut_slice());

        let mut clients = Vec::new();

        for encoded_client in buffer.chunks_exact(12) {
            clients.push(Client::decode(encoded_client)?)
        }

        Ok(clients)
    }
}
