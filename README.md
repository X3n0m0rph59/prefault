# prefault Project

A leightweight tool to pre-fault pages from often used files, ahead of time.
Supports locking of core libraries into memory to avoid eviction of important pages, which may cause a laggy user experience.

## Advantages over `precached`

* Much more leightweight
* Much smaller code base
* Saves power, since it does not require any calculations, once the process snapshots have been created
* Does not need a full fledged daemon

## Disadvantages

* More or less static, does not adapt to system behaviour changes

