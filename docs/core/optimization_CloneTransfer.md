# Clone Transfers

Sending data over the network is the biggest bottleneck for quick file transfer. To minimize this, we can isolate files that the receiver already owns; saving bandwidth. This requires some form of identification and verification of files in the manifest against files present in the receiver's preferred download location. 

Unfortunately, verification of each file whose file path matches a location in local storage by hashing the local copy and comparing the provided file hash is a slow process, to mitigate the delay caused by this identification process we can introduce a file hash cache. The file hash cache is a global storage of all files that Beacon has received in the past 7 days along with each file's `mtime` which is an indicator of last modification time and the size. 

## Identification

Upon reception of the transfer manifest, the receiver has a list of all files it expects, their size and their `mtime`. Beacon's runtime initializes a crawler which collects a list of all files in the manifest that do not have a local copy with the same file path and file size, this is the inital `WANT` list that the receiver passes to the sender, at the same time, it maintains a list of all files with matching metadata that await hash and `mtime` verification. The sender can now begin processing all files specified by the `WANT` list, and shoot them across the network to the receiver.

## Verification

Beacon must run a second verification stage on the list of local copies collected during the Identification step which runs parallely with the transimission. This step is necessary to ensure the files in local storage contain the same data that the sender wishes to transmit. Beacon verifies the validity of the file by comparing its hash against the file hash from the manifest. In case of a mismatch, the selected files are added to another list which is sent in a `WANT` request. All files that match are used to update the local bitmap of owned files. 

## Hash Cache

Hashing every file with a matching file path can be quite slow, so Beacon maintains a global hash cache of all files it has received in the last 7 days. This speeds up verification considerably, since it replaces heavy computation with a simple database lookup. However, in case the hash cache is stale (`mtime` of the cache does not match the manifest) or simply absent, Beacon must hash the file again, which is handled by the verification stage.

![Identification and Validation](../assets/verification.png)