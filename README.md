# pglite

Embedded Postgres database

## Building

```sh-session
$ git submodule update --init   # setup postgres submodule
$ ./prepare-postgres.sh          # rewrite postgres source code to use thread-local storage
$ cargo build                   # build everything
```
