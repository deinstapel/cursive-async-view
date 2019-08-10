# Creating a new GIF

## Recording

Inside a `80x24` terminal record it using

```
$ cargo build --example simple
$ cd assets
$ terminalizer record --config ./config.yml async-view-loading
```

## Rendering

```
$ terminalizer render async-view-loading.yml -o async-view-loading.gif
```

## Optimizing

```
$ gifsicle --colors 64 -O3 async-view-loading.gif -o async-view-loading.gif
```
