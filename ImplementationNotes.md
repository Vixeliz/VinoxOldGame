// Networking Responsibilities
Server is responsible for creating the actual chunk data. Therefore we will handle that in the server crate.
However we still obviously need to send it to the client so the client can mesh it.

Another thing is when chunks are updated on the server from players breaking or placing blocks we will only send changed blocks to clients
Basically acting as if we were adding or removing that block on the client. This will include and palette changes. Perhaps palette should just always be sent

// Scripting
The scripting api at first will be very barebones. I plan to only support the most simple task such as modifying
the world blocks and doing simple things relating to entities such as health. I don't see this being an issue
as of now. Most base content I add will be either done in rust for things I consider critical to base game.
And most blocks and entities only need some ron files to describe them with stuff such as ai mostly handled in
rust.

// Handling Chunks
Chunks will be put into a queue and spread across frames with bevys asynccomputetaskpool. This goes for both
meshes on the client and the actual chunk data on the server.