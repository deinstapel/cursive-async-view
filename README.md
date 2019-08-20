<h1 align="center">Welcome to cursive-async-view 👋</h1>
<p align="center">
  <a href="https://travis-ci.org/deinstapel/cursive-async-view">
    <img src="https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fdeinstapel.github.io%2Fcursive-async-view%2Fstable-build.json" alt="stable build">
  </a>
  <a href="https://travis-ci.org/deinstapel/cursive-async-view">
    <img src="https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fdeinstapel.github.io%2Fcursive-async-view%2Fnightly-build.json" alt="nightly build">
  </a>
  <a href="https://crates.io/crates/cursive-async-view">
    <img alt="crates.io" src="https://img.shields.io/crates/v/cursive-async-view.svg">
  </a>
  <a href="https://docs.rs/cursive-async-view">
    <img alt="Docs.rs" src="https://docs.rs/cursive-async-view/badge.svg">
  </a>
  <a href="https://github.com/deinstapel/cursive-async-view/blob/master/LICENSE">
    <img alt="GitHub" src="https://img.shields.io/github/license/deinstapel/cursive-async-view.svg">
  </a>
  <a href="http://makeapullrequest.com">
    <img alt="PRs Welcome" src="https://img.shields.io/badge/PRs-welcome-brightgreen.svg">
  </a>
  <br>
  <i>A loading-screen wrapper for
  <a href="https://github.com/gyscos/cursive">gyscos/cursive</a>
  views</i>
</p>

---

> This project is work-in-progress

This project provides a wrapper view with a loading screen for [gyscos/cursive](https://github.com/gyscos/cursive) views. The loading screen will disappear once the wrapped view is fully loaded. This is useful for displaying views which may take long to construct or depend on e.g. the network.

## How does it look like? `demo` [![terminalizer](https://img.shields.io/badge/GIF-terminalizer-blueviolet.svg)](https://github.com/faressoft/terminalizer)

<details>
  <summary>Expand to view</summary>
  <img src="assets/async-view-loading.gif" alt="async-view-loading demo">
  <img src="assets/async-view-progress.gif" alt="async-view-progress demo">
</details>

## Usage

Simply add to your `Cargo.toml`

```toml
[dependencies]
cursive-async-view = "^0"
```

## Troubleshooting

If you find any bugs/unexpected behaviour or you have a proposition for future changes open an issue describing the current behaviour and what you expected.

## Development [![cargo test](https://img.shields.io/endpoint.svg?url=https%3A%2F%2Fdeinstapel.github.io%2Fcursive-async-view%2Fcargo-test.json)](https://travis-ci.org/deinstapel/cursive-async-view)

> TBD

### Running the tests

Just run

```
$ cargo test
```

to execute all available tests.

#### shields.io endpoints

[shields.io](https://shields.io) endpoints are generated inside the `./target/shields` folder. They are used in this README.

## Authors

**Fin Christensen**

> [:octocat: `@fin-ger`](https://github.com/fin-ger)  
> [:elephant: `@fin_ger@mastodon.social`](https://mastodon.social/web/accounts/787945)  
> [:bird: `@fin_ger_github`](https://twitter.com/fin_ger_github)  

<br>

**Johannes Wünsche**

> [:octocat: `@jwuensche`](https://github.com/jwuensche)  
> [:elephant: `@fredowald@mastodon.social`](https://mastodon.social/web/accounts/843376)  
> [:bird: `@Fredowald`](https://twitter.com/fredowald)  

## Show your support

Give a :star: if this project helped you!
