# P2P UDP File-Sharing Solution
This document describes a concept implementation of a peer to peer file transferring solution which uses a Server to resolve an UDP connection between the peers.

## Components
This project will consist of two components:
  1. Client Application
  2. Connection Resolver (Most likely a [STUN](https://en.wikipedia.org/wiki/STUN) server)
  3. Protocol for Transferring Files over UDP

### Client Application

The client application will provide a user interface which is used both to share and receive files. If the user desires to share a file, they must specify the file in question. They will then recieve a code which they can forward to desired recipient. The recipient can then enter this code to establish the connection with the sender and download the shared file.

### Connection Resolver
This should already be a prebuilt solution, should this project use STUN. There are [a lot of public STUN servers](https://gist.github.com/mondain/b0ec1cf5f60ae726202e) which could be used to resolve the connection, all the users would have to do is share the same one. 

An alternative would be hosting one exclusively for the project. 

### Protocol for Transferring Files over UDP
This is an important component of this project. Should the protocol not account for the natural unreliability of UDP, data loss may occur, therefore the transfer of a file should always account for this. Data corruption should also be accounted for.

## Challenges
A certain difficulty will be the development of a reliable protocol library with low overhead and high transfer reliability. 

There are many technologies which are new in this project(at least to me). Learning how to use them will also pose a challenge.

Further challenges may arise as we approach the project further.
