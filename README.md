# Welcome to the Prefault Project

[![Copr build status](https://copr.fedorainfracloud.org/coprs/x3n0m0rph59/prefault/package/prefault-git/status_image/last_build.png)](https://copr.fedorainfracloud.org/coprs/x3n0m0rph59/prefault/package/prefault-git/)

## What is prefault?

#### TL;DR

Pre-fault and optionally lock files into the kernel's page cache to improve application startup times and reduce desktop lagging.

Prefault is a lightweight tool used to pre-fault and optionally mlock() pages from often used files into memory, ahead of time.
You may use prefault to lock core system binaries into memory, to avoid eviction of important pages during memory pressure, which would then cause a 'laggy' user experience.

## Quick Installation Guide

### Install on Arch Linux and Manjaro

```shell
    $ yay prefault-git
```

### Install on Fedora

```shell
    $ sudo dnf copr enable x3n0m0rph59/prefault-git
    $ sudo dnf install prefault-git
```

## Install from Source

    $ git clone https://github.com/X3n0m0rph59/prefault.git
    $ cd prefault/
    $ cargo build --release

    # ... copy files ...

## How does prefault compare to precached?

### Advantages over `precached`

* Much more lightweight
* Much smaller code base
* Saves power, since it does not require any calculations, once the process snapshots have been created
* Does not need a full fledged daemon

### Disadvantages

* More or less static, does not adapt to system behavior changes
* Manual intervention required if updates of a package introduce changes to the dependencies of a binary

## Authors

prefault - Copyright (C) 2019-2020 the prefault developers
