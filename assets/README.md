# Creating a new GIF

## Recording

Inside a `80x24` terminal record it using

```
$ cargo build --example simple
$ cd assets
$ terminalizer record --config ./config.yml async-view-simple
```

> As xterm.js is still unable to render emojis properly (geez, it's 2019...) insert a space
> after all emojis in the recording manually to workaround wide-character issues

## Rendering

```
$ terminalizer render async-view-simple.yml -o async-view-simple.gif
```

## Optimizing

```
$ gifsicle --colors 32 -O3 async-view-simple.gif -o async-view-simple.gif
```
