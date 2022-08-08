

/* How the file is sent
 * --------------------
 * The FIRST PACKET sent contains the following metadata:
 * Status: 1 byte; UftServerStatus::FILE_META
 * File name: UTF-8 string; 127 bytes maximum (excluding zero-terminator)
 * Packets needed: 8 bytes unsigned integer
 * Final packet data size: 2 bytes unsigned integer
 * Reserved: 22 bytes (for future use)
 * 
 * It is required that the client responds with a packet of THIS FORMAT:
 * Status: 1 byte; UftClientStatus::META_RECEIVED
 * Received Data Hash: 32 bytes
 * The server must check that the hash is correct.
 * 
 * If the hash is incorrect, then THIS PACKET must be returned:
 * Status: 1 byte; UftServerStatus::SERVER_ERROR
 * Error code: 4 bytes; UftServerError::META_HASH_INVALID
 * And communication is terminated. The client can then retry the connection.
 * 
 * If the hash is correct, then communication can begin. Every packet containing data must have THIS FORMAT:
 * Status: 1 byte; UftServerStatus::FILE_DATA
 * Packet Number: 8 bytes unsigned integer
 * Packet Data: UFT_DATA_SIZE bytes
 * The client must recognize the final packet and only extract the remaining data from it.
 * 
 * TODO: Write this more precisely
 * It must then announce that it has finished receiving the file, or list the missing blocks in a list. If blocks were missing, the server will provide them,
 * and once the client has confirmed that it has received the blocks, it will then send a hash to check if the file was properly transferred. If the hash doesn't match,
 * The client will send an error and the communication is terminated. The client can then try again. Should the hash match, it will confirm that the hash was valid and 
 * end the connection.
 */

// Protocol configuration constants
pub const UFT_BUFFER_SIZE: usize = 4096;
pub const UFT_SERVER_MAX_SYM: usize = 16;
pub const UTF_SERVER_MAX_LISTENER_BLOCKS: usize = 64;

// The first 4 bits are reserved for client status
#[repr(u8)]
pub enum UftClientStatus {
    FILE_REQUEST = 0,
    META_RECEIVED,
    BLOCK_REQUEST,
    FILE_RECEIVED,
    HASH_VALID,
    CLIENT_ERROR = 0x0F
}

#[repr(u32)]
pub enum UftClientError {
    FILE_HASH_INVALID = 0, // The hash of the received file does not match the hash sent for validation
}

// The latter 4 bits are reserved for server status
#[repr(u8)]
pub enum UftServerStatus{
    FILE_META = 1 << 4,
    FILE_DATA,
    FILE_COMPLETE,
    SERVER_ERROR = 0x0F
}

#[repr(u32)]
pub enum UftServerError {
    META_HASH_INVALID = 0, // The received hash of the file metadata does not match the original
}

pub const UFT_DATA_SIZE: usize = UFT_BUFFER_SIZE - 9;