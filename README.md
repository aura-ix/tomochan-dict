# tomochan-dict
**NOTE:** The disk format is currently not remotely stable, as this is currently in an exploratory proof of concept stage.

This repository provides the implementation for the tomochan dictionary format, with the following features:

- Ability to import any yomitan v3 dictionary (current version)
- Virtually instant imports
- Modifying the order of dictionaries, disabling, and removing dictionaries is virtually instant
- Sub-millisecond lookups even when using multiple dictionaries
- `mmap`able disk format, meaning low minimum required memory usage, but able to take advantage of extra memory for speed improvements as controlled by OS memory pressure
- Small dictionary files (generally speaking the on disk representation of a tomochan dictionary file will be the same size or smaller than the yomitan import dictionary format)

This project was created in response to the following issues with yomitan:

- Using the recommended set of japanese dictionaries takes ~10GB of disk space with yomitan (compared to ~0.7GB with tomochan-dict)
- Importing dictionaries takes a very long time (10+ minutes, compared to ~200ms)
- Reorganizing dictionaries can take a similar amount of time to importing them (no penalty in tomochan-dict)
- The yomitan API routinely takes >20ms to respond, and often times several hundred milliseconds on macos

In comparison to yomitan, this means you need less than a tenth of the disk space for the same dictionaries, and importing is now very fast.

# Approach
As opposed to using indexeddb in the browser, tomochan dictionaries implement a straightforward compressed key value store using a single unified finite state transducer and a seekable zstd archive.

The lookup process is roughly as follows:
- Depending on the type of data you are searching for in the dictionary (terms, term metadata, kanji, media files), a type key is prepended to prepended to your query (enabling one FST to be used for all data as a generic KV store index)
- The FST is queried for the key, and any matching values will be returned in the form of an offset into the zstd archive
- The zstd archive seeks to the relevant position, and reads the serialized dictionary data back out, returning it to the user

# TODO
- Conversion tool
    - Only include media files that are directly referenced by the dictionary data
    - Consider reencoding images
    - When encoding media into the zstd stream, only use the zstd stream if it actually compresses to be smaller
        - This might not be worth the file size reduction because then we would have to split our unified store into zstd data and uncompressed data
- Fix the crate structure to be usable as a library and expose a command line tool separately
- Support deconjugation with configurable data files (ex. from yomitan, nazeka) 
- Proper testing
- Migrate away from bincode to a maintained alternative
- Properly integrate the mmap feature for fst (especially in the case of )
- File header with magic, schema version, checksum
- Implement some form of caching wrt. zstd stream decompression
    - May be better to just ignore this as it will require configuration around how many cache entries to keep, and lookup is already quite fast by UX terms even if it is not fast by dictionary implementation terms
- Look into zstd seekable encoder/decoder options
- Look into bincode options
- Look into forking fst for further size improvement