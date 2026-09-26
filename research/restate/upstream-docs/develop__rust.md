<!DOCTYPE html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1.0"><meta name="generator" content="rustdoc"><meta name="description" content="Restate Rust SDK"><title>restate_sdk - Rust</title><script>if(window.location.protocol!=="file:")document.head.insertAdjacentHTML("beforeend","SourceSerif4-Regular-6b053e98.ttf.woff2,FiraSans-Italic-81dc35de.woff2,FiraSans-Regular-0fe48ade.woff2,FiraSans-MediumItalic-ccf7e434.woff2,FiraSans-Medium-e1aa3f0a.woff2,SourceCodePro-Regular-8badfe75.ttf.woff2,SourceCodePro-Semibold-aa29a496.ttf.woff2".split(",").map(f=>`<link rel="preload" as="font" type="font/woff2"href="/-/rustdoc.static/${f}">`).join(""))</script><link rel="stylesheet" href="/-/rustdoc.static/normalize-9960930a.css"><link rel="stylesheet" href="/-/static/vendored.css?0-0-0-9439cafc1e2ea4a64558a489c81351cd959d6077-2026-09-22" media="all" /><link rel="stylesheet" href="/-/rustdoc.static/rustdoc-203241e4.css"><meta name="rustdoc-vars" data-root-path="../" data-static-root-path="/-/rustdoc.static/" data-current-crate="restate_sdk" data-themes="" data-resource-suffix="-20260921-1.100.0-nightly-1303417c4" data-rustdoc-version="1.100.0-nightly (1303417c4 2026-09-21)" data-channel="nightly" data-search-js="search-b167f383.js" data-stringdex-js="stringdex-b5109af9.js" data-settings-js="settings-170eb4bf.js" ><script src="/-/rustdoc.static/storage-41dd4d93.js"></script><script defer src="../crates-20260921-1.100.0-nightly-1303417c4.js"></script><script defer src="/-/rustdoc.static/main-1814ead3.js"></script><noscript><link rel="stylesheet" href="/-/rustdoc.static/noscript-9f56287d.css"></noscript><link rel="alternate icon" type="image/png" href="/-/rustdoc.static/favicon-32x32-eab170b8.png"><link rel="icon" type="image/svg+xml" href="/-/rustdoc.static/favicon-044be391.svg"><link rel="stylesheet" href="/-/static/rustdoc-2025-08-20.css?0-0-0-9439cafc1e2ea4a64558a489c81351cd959d6077-2026-09-22" media="all" /><link rel="stylesheet" href="/-/static/font-awesome.css?0-0-0-9439cafc1e2ea4a64558a489c81351cd959d6077-2026-09-22" media="all" />

<link rel="search" href="/-/static/opensearch.xml" type="application/opensearchdescription+xml" title="Docs.rs" />

<script type="text/javascript">(function() {
    function applyTheme(theme) {
        if (theme) {
            document.documentElement.dataset.docsRsTheme = theme;
        }
    }

    window.addEventListener("storage", ev => {
        if (ev.key === "rustdoc-theme") {
            applyTheme(ev.newValue);
        }
    });

    // see ./storage-change-detection.html for details
    window.addEventListener("message", ev => {
        if (ev.data && ev.data.storage && ev.data.storage.key === "rustdoc-theme") {
            applyTheme(ev.data.storage.value);
        }
    });

    applyTheme(window.localStorage.getItem("rustdoc-theme"));
})();</script></head><body class="rustdoc-page">
<div class="nav-container">
    <div class="container">
        <div class="pure-menu pure-menu-horizontal" role="navigation" aria-label="Main navigation">
            <form action="/releases/search"
                  method="GET"
                  id="nav-search-form"
                  class="landing-search-form-nav  ">

                
                <a href="/" class="pure-menu-heading pure-menu-link docsrs-logo" aria-label="Docs.rs">
                    <span title="Docs.rs"><span class="fa fa-solid fa-cubes " aria-hidden="true"></span></span>
                    <span class="title">Docs.rs</span>
                </a><ul class="pure-menu-list">
    <script id="crate-metadata" type="application/json">
        
        {
            "name": "restate-sdk",
            "version": "0.12.1"
        }
    </script><li class="pure-menu-item pure-menu-has-children crate-dropdown">
            <a href="#" class="pure-menu-link crate-name" title="Restate SDK for Rust">
                <span class="fa fa-solid fa-cube " aria-hidden="true"></span>
                <span class="title">restate-sdk-0.12.1</span>
            </a><div class="pure-menu-children package-details-menu">
                
                <ul class="pure-menu-list menu-item-divided">
                    <li class="pure-menu-heading" id="crate-title">
                        restate-sdk 0.12.1
                        <span id="clipboard" class="svg-clipboard" title="Copy crate name and version information"></span>
                    </li><li class="pure-menu-item">
                        <a href="/restate-sdk/0.12.1/restate_sdk/" class="pure-menu-link description" id="permalink" title="Get a link to this specific version"><span class="fa fa-solid fa-link " aria-hidden="true"></span> Permalink
                        </a>
                    </li><li class="pure-menu-item">
                        <a href="/crate/restate-sdk/latest" class="pure-menu-link description" title="See restate-sdk in docs.rs">
                            <span class="fa fa-solid fa-cube " aria-hidden="true"></span> Docs.rs crate page
                        </a>
                    </li><li class="pure-menu-item">
                            <span class="pure-menu-link description license"><span class="fa fa-solid fa-scale-unbalanced-flip " aria-hidden="true"></span>
                            <a href="https://spdx.org/licenses/MIT" class="pure-menu-sublink">MIT</a></span>
                        </li><li class="pure-menu-item">
                            <span class="pure-menu-link description" title="Built with rustc 1.100.0-nightly (1303417c4 2026-09-21)">
                                <span class="fa fa-solid fa-gears " aria-hidden="true"></span> 22 September 2026
                            </span>
                        </li></ul>

                <div class="pure-g menu-item-divided">
                    <div class="pure-u-1-2 right-border">
                        <ul class="pure-menu-list">
                            <li class="pure-menu-heading">Links</li>

                            <li class="pure-menu-item">
                                    <a href="https://github.com/restatedev/sdk-rust" class="pure-menu-link">
                                        <span class="fa fa-solid fa-code-branch " aria-hidden="true"></span> Repository
                                    </a>
                                </li><li class="pure-menu-item">
                                <a href="https://crates.io/crates/restate-sdk" class="pure-menu-link" title="See restate-sdk in crates.io">
                                    <span class="fa fa-solid fa-cube " aria-hidden="true"></span> crates.io
                                </a>
                            </li>

                            
                            <li class="pure-menu-item">
                                <a href="/crate/restate-sdk/latest/source/" title="Browse source of restate-sdk-0.12.1" class="pure-menu-link">
                                    <span class="fa fa-solid fa-folder-open " aria-hidden="true"></span> Source
                                </a>
                            </li>
                        </ul>
                    </div><div class="pure-u-1-2">
                        <ul class="pure-menu-list" id="topbar-owners">
                            <li class="pure-menu-heading">Owners</li><li class="pure-menu-item">
                                    <a href="https://crates.io/users/slinkydeveloper" class="pure-menu-link">
                                        <span class="fa fa-solid fa-user " aria-hidden="true"></span> slinkydeveloper
                                    </a>
                                </li><li class="pure-menu-item">
                                    <a href="https://crates.io/teams/github:restatedev:owners" class="pure-menu-link">
                                        <span class="fa fa-solid fa-user " aria-hidden="true"></span> github:restatedev:owners
                                    </a>
                                </li></ul>
                    </div>
                </div>

                <div class="pure-g menu-item-divided">
                    <div class="pure-u-1-2 right-border">
                        <ul class="pure-menu-list">
                            <li class="pure-menu-heading">Dependencies</li>

                            
                            <li class="pure-menu-item">
                                <div class="pure-menu pure-menu-scrollable sub-menu" tabindex="-1">
                                    <ul class="pure-menu-list">
                                        <li class="pure-menu-item"><a href="/aws_lambda_events/^1.0/" class="pure-menu-link">
                aws_lambda_events ^1.0
                
                    <i class="dependencies normal">normal</i>
                    
                        <i>optional</i>
                    
                
            </a>
        </li><li class="pure-menu-item"><a href="/bytes/^1.11/" class="pure-menu-link">
                bytes ^1.11
                
                    <i class="dependencies normal">normal</i>
                    
                
            </a>
        </li><li class="pure-menu-item"><a href="/futures/^0.3/" class="pure-menu-link">
                futures ^0.3
                
                    <i class="dependencies normal">normal</i>
                    
                
            </a>
        </li><li class="pure-menu-item"><a href="/hickory-resolver/^0.26/" class="pure-menu-link">
                hickory-resolver ^0.26
                
                    <i class="dependencies normal">normal</i>
                    
                        <i>optional</i>
                    
                
            </a>
        </li><li class="pure-menu-item"><a href="/hostname/^0.4/" class="pure-menu-link">
                hostname ^0.4
                
                    <i class="dependencies normal">normal</i>
                    
                        <i>optional</i>
                    
                
            </a>
        </li><li class="pure-menu-item"><a href="/http/^1.4/" class="pure-menu-link">
                http ^1.4
                
                    <i class="dependencies normal">normal</i>
                    
                
            </a>
        </li><li class="pure-menu-item"><a href="/http-body/^1.0.1/" class="pure-menu-link">
                http-body ^1.0.1
                
                    <i class="dependencies normal">normal</i>
                    
                
            </a>
        </li><li class="pure-menu-item"><a href="/http-body-util/^0.1/" class="pure-menu-link">
                http-body-util ^0.1
                
                    <i class="dependencies normal">normal</i>
                    
                
            </a>
        </li><li class="pure-menu-item"><a href="/http-serde/^2.1.1/" class="pure-menu-link">
                http-serde ^2.1.1
                
                    <i class="dependencies normal">normal</i>
                    
                        <i>optional</i>
                    
                
            </a>
        </li><li class="pure-menu-item"><a href="/hyper/^1.8/" class="pure-menu-link">
                hyper ^1.8
                
                    <i class="dependencies normal">normal</i>
                    
                        <i>optional</i>
                    
                
            </a>
        </li><li class="pure-menu-item"><a href="/hyper-util/^0.1/" class="pure-menu-link">
                hyper-util ^0.1
                
                    <i class="dependencies normal">normal</i>
                    
                        <i>optional</i>
                    
                
            </a>
        </li><li class="pure-menu-item"><a href="/jsonwebtoken/^10.3/" class="pure-menu-link">
                jsonwebtoken ^10.3
                
                    <i class="dependencies normal">normal</i>
                    
                        <i>optional</i>
                    
                
            </a>
        </li><li class="pure-menu-item"><a href="/lambda_runtime/^1.0/" class="pure-menu-link">
                lambda_runtime ^1.0
                
                    <i class="dependencies normal">normal</i>
                    
                        <i>optional</i>
                    
                
            </a>
        </li><li class="pure-menu-item"><a href="/percent-encoding/^2.3/" class="pure-menu-link">
                percent-encoding ^2.3
                
                    <i class="dependencies normal">normal</i>
                    
                
            </a>
        </li><li class="pure-menu-item"><a href="/pin-project-lite/^0.2/" class="pure-menu-link">
                pin-project-lite ^0.2
                
                    <i class="dependencies normal">normal</i>
                    
                
            </a>
        </li><li class="pure-menu-item"><a href="/rand/^0.10/" class="pure-menu-link">
                rand ^0.10
                
                    <i class="dependencies normal">normal</i>
                    
                        <i>optional</i>
                    
                
            </a>
        </li><li class="pure-menu-item"><a href="/regress/^0.10/" class="pure-menu-link">
                regress ^0.10
                
                    <i class="dependencies normal">normal</i>
                    
                
            </a>
        </li><li class="pure-menu-item"><a href="/reqwest/^0.13/" class="pure-menu-link">
                reqwest ^0.13
                
                    <i class="dependencies normal">normal</i>
                    
                        <i>optional</i>
                    
                
            </a>
        </li><li class="pure-menu-item"><a href="/restate-sdk-macros/^0.12/" class="pure-menu-link">
                restate-sdk-macros ^0.12
                
                    <i class="dependencies normal">normal</i>
                    
                
            </a>
        </li><li class="pure-menu-item"><a href="/restate-sdk-shared-core/=7.0.3/" class="pure-menu-link">
                restate-sdk-shared-core =7.0.3
                
                    <i class="dependencies normal">normal</i>
                    
                
            </a>
        </li><li class="pure-menu-item"><a href="/rustls/^0.23/" class="pure-menu-link">
                rustls ^0.23
                
                    <i class="dependencies normal">normal</i>
                    
                        <i>optional</i>
                    
                
            </a>
        </li><li class="pure-menu-item"><a href="/rustls-native-certs/^0.8/" class="pure-menu-link">
                rustls-native-certs ^0.8
                
                    <i class="dependencies normal">normal</i>
                    
                        <i>optional</i>
                    
                
            </a>
        </li><li class="pure-menu-item"><a href="/schemars/^1.2/" class="pure-menu-link">
                schemars ^1.2
                
                    <i class="dependencies normal">normal</i>
                    
                        <i>optional</i>
                    
                
            </a>
        </li><li class="pure-menu-item"><a href="/serde/^1.0/" class="pure-menu-link">
                serde ^1.0
                
                    <i class="dependencies normal">normal</i>
                    
                
            </a>
        </li><li class="pure-menu-item"><a href="/serde_json/^1.0/" class="pure-menu-link">
                serde_json ^1.0
                
                    <i class="dependencies normal">normal</i>
                    
                
            </a>
        </li><li class="pure-menu-item"><a href="/socket2/^0.6/" class="pure-menu-link">
                socket2 ^0.6
                
                    <i class="dependencies normal">normal</i>
                    
                        <i>optional</i>
                    
                
            </a>
        </li><li class="pure-menu-item"><a href="/thiserror/^2.0/" class="pure-menu-link">
                thiserror ^2.0
                
                    <i class="dependencies normal">normal</i>
                    
                
            </a>
        </li><li class="pure-menu-item"><a href="/tokio/^1.49/" class="pure-menu-link">
                tokio ^1.49
                
                    <i class="dependencies normal">normal</i>
                    
                
            </a>
        </li><li class="pure-menu-item"><a href="/tokio-rustls/^0.26/" class="pure-menu-link">
                tokio-rustls ^0.26
                
                    <i class="dependencies normal">normal</i>
                    
                        <i>optional</i>
                    
                
            </a>
        </li><li class="pure-menu-item"><a href="/tokio-util/^0.7/" class="pure-menu-link">
                tokio-util ^0.7
                
                    <i class="dependencies normal">normal</i>
                    
                        <i>optional</i>
                    
                
            </a>
        </li><li class="pure-menu-item"><a href="/tracing/^0.1/" class="pure-menu-link">
                tracing ^0.1
                
                    <i class="dependencies normal">normal</i>
                    
                
            </a>
        </li><li class="pure-menu-item"><a href="/tracing-subscriber/^0.3/" class="pure-menu-link">
                tracing-subscriber ^0.3
                
                    <i class="dependencies normal">normal</i>
                    
                        <i>optional</i>
                    
                
            </a>
        </li><li class="pure-menu-item"><a href="/url/^2.5/" class="pure-menu-link">
                url ^2.5
                
                    <i class="dependencies normal">normal</i>
                    
                        <i>optional</i>
                    
                
            </a>
        </li><li class="pure-menu-item"><a href="/uuid/^1.20/" class="pure-menu-link">
                uuid ^1.20
                
                    <i class="dependencies normal">normal</i>
                    
                        <i>optional</i>
                    
                
            </a>
        </li><li class="pure-menu-item"><a href="/base64/^0.22/" class="pure-menu-link">
                base64 ^0.22
                
                    <i class="dependencies dev">dev</i>
                    
                
            </a>
        </li><li class="pure-menu-item"><a href="/bs58/^0.5/" class="pure-menu-link">
                bs58 ^0.5
                
                    <i class="dependencies dev">dev</i>
                    
                
            </a>
        </li><li class="pure-menu-item"><a href="/ed25519-dalek/^2/" class="pure-menu-link">
                ed25519-dalek ^2
                
                    <i class="dependencies dev">dev</i>
                    
                
            </a>
        </li><li class="pure-menu-item"><a href="/hyper/^1.8/" class="pure-menu-link">
                hyper ^1.8
                
                    <i class="dependencies dev">dev</i>
                    
                
            </a>
        </li><li class="pure-menu-item"><a href="/rand/^0.10/" class="pure-menu-link">
                rand ^0.10
                
                    <i class="dependencies dev">dev</i>
                    
                
            </a>
        </li><li class="pure-menu-item"><a href="/reqwest/^0.13/" class="pure-menu-link">
                reqwest ^0.13
                
                    <i class="dependencies dev">dev</i>
                    
                
            </a>
        </li><li class="pure-menu-item"><a href="/schemars/^1.2/" class="pure-menu-link">
                schemars ^1.2
                
                    <i class="dependencies dev">dev</i>
                    
                
            </a>
        </li><li class="pure-menu-item"><a href="/tokio/^1/" class="pure-menu-link">
                tokio ^1
                
                    <i class="dependencies dev">dev</i>
                    
                
            </a>
        </li><li class="pure-menu-item"><a href="/tracing-subscriber/^0.3/" class="pure-menu-link">
                tracing-subscriber ^0.3
                
                    <i class="dependencies dev">dev</i>
                    
                
            </a>
        </li><li class="pure-menu-item"><a href="/trybuild/^1.0/" class="pure-menu-link">
                trybuild ^1.0
                
                    <i class="dependencies dev">dev</i>
                    
                
            </a>
        </li><li class="pure-menu-item"><a href="/jsonptr/^0.7/" class="pure-menu-link">
                jsonptr ^0.7
                
                    <i class="dependencies build">build</i>
                    
                
            </a>
        </li><li class="pure-menu-item"><a href="/prettyplease/^0.2/" class="pure-menu-link">
                prettyplease ^0.2
                
                    <i class="dependencies build">build</i>
                    
                
            </a>
        </li><li class="pure-menu-item"><a href="/serde_json/^1.0/" class="pure-menu-link">
                serde_json ^1.0
                
                    <i class="dependencies build">build</i>
                    
                
            </a>
        </li><li class="pure-menu-item"><a href="/syn/^2.0/" class="pure-menu-link">
                syn ^2.0
                
                    <i class="dependencies build">build</i>
                    
                
            </a>
        </li><li class="pure-menu-item"><a href="/typify/^0.6/" class="pure-menu-link">
                typify ^0.6
                
                    <i class="dependencies build">build</i>
                    
                
            </a>
        </li>
                                    </ul>
                                </div>
                            </li>
                        </ul>
                    </div>

                    <div class="pure-u-1-2">
                        <ul class="pure-menu-list">
                            <li class="pure-menu-heading">Versions</li>

                            <li class="pure-menu-item">
                                <div class="pure-menu pure-menu-scrollable sub-menu" id="releases-list" tabindex="-1" data-url="/crate/restate-sdk/latest/menus/releases/restate_sdk/">
                                    <span class="rotate"><span class="fa fa-solid fa-spinner " aria-hidden="true"></span></span>
                                </div>
                            </li>
                        </ul>
                    </div>
                </div>
                    
                    
                    <div class="pure-g">
                        <div class="pure-u-1">
                            <ul class="pure-menu-list">
                                <li>
                                    <a href="/crate/restate-sdk/latest" class="pure-menu-link">
                                        <b>68.31%</b>
                                        of the crate is documented
                                    </a>
                                </li>
                            </ul>
                        </div>
                    </div></div>
        </li><li class="pure-menu-item pure-menu-has-children">
                <a href="#" class="pure-menu-link" aria-label="Platform">
                    <span class="fa fa-solid fa-gears " aria-hidden="true"></span>
                    <span class="title">Platform</span>
                </a>

                
                <ul class="pure-menu-children" id="platforms" data-url="/crate/restate-sdk/latest/menus/platforms/restate_sdk/"><li class="pure-menu-item">
            <a href="/crate/restate-sdk/latest/target-redirect/restate_sdk/" class="pure-menu-link" data-fragment="retain" rel="nofollow">x86_64-unknown-linux-gnu</a>
        </li></ul>
            </li><li class="pure-menu-item">
                <a href="/crate/restate-sdk/latest/features" title="Browse available feature flags of restate-sdk-0.12.1" class="pure-menu-link">
                    <span class="fa fa-solid fa-flag " aria-hidden="true"></span>
                    <span class="title">Feature flags</span>
                </a>
            </li>
        
    
</ul><div class="spacer"></div>
                
                <div id="abnormalities" class="hidden"></div>

                <ul class="pure-menu-list">
                    <li class="pure-menu-item pure-menu-has-children">
                        <a href="#" class="pure-menu-link" aria-label="docs.rs">docs.rs</a>
                        <ul class="pure-menu-children aligned-icons"><li class="pure-menu-item"><a class="pure-menu-link" href="/about"><span class="fa fa-solid fa-circle-info " aria-hidden="true"></span> About docs.rs</a></li><li class="pure-menu-item"><a class="pure-menu-link" href="/about/badges"><span class="fa fa-brands fa-fonticons " aria-hidden="true"></span> Badges</a></li><li class="pure-menu-item"><a class="pure-menu-link" href="/about/builds"><span class="fa fa-solid fa-gears " aria-hidden="true"></span> Builds</a></li><li class="pure-menu-item"><a class="pure-menu-link" href="/about/metadata"><span class="fa fa-solid fa-table " aria-hidden="true"></span> Metadata</a></li><li class="pure-menu-item"><a class="pure-menu-link" href="/about/redirections"><span class="fa fa-solid fa-road " aria-hidden="true"></span> Shorthand URLs</a></li><li class="pure-menu-item"><a class="pure-menu-link" href="/about/download"><span class="fa fa-solid fa-download " aria-hidden="true"></span> Download</a></li><li class="pure-menu-item"><a class="pure-menu-link" href="/about/rustdoc-json"><span class="fa fa-solid fa-file-code " aria-hidden="true"></span> Rustdoc JSON</a></li><li class="pure-menu-item"><a class="pure-menu-link" href="/releases/queue"><span class="fa fa-solid fa-gears " aria-hidden="true"></span> Build queue</a></li><li class="pure-menu-item"><a class="pure-menu-link" href="https://foundation.rust-lang.org/policies/privacy-policy/#docs.rs" target="_blank"><span class="fa fa-solid fa-shield-halved " aria-hidden="true"></span> Privacy policy</a></li>
                        </ul>
                    </li>
                </ul>
                <ul class="pure-menu-list"><li class="pure-menu-item pure-menu-has-children">
                        <a href="#" class="pure-menu-link" aria-label="Rust">Rust</a>
                        <ul class="pure-menu-children">
                            <li class="pure-menu-item"><a class="pure-menu-link" href="https://www.rust-lang.org/" target="_blank">Rust website</a></li>
                            <li class="pure-menu-item"><a class="pure-menu-link" href="https://doc.rust-lang.org/book/" target="_blank">The Book</a></li>

                            <li class="pure-menu-item"><a class="pure-menu-link" href="https://doc.rust-lang.org/std/" target="_blank">Standard Library API Reference</a></li>

                            <li class="pure-menu-item"><a class="pure-menu-link" href="https://doc.rust-lang.org/rust-by-example/" target="_blank">Rust by Example</a></li>

                            <li class="pure-menu-item"><a class="pure-menu-link" href="https://doc.rust-lang.org/cargo/guide/" target="_blank">The Cargo Guide</a></li>

                            <li class="pure-menu-item"><a class="pure-menu-link" href="https://doc.rust-lang.org/nightly/clippy" target="_blank">Clippy Documentation</a></li>
                        </ul>
                    </li>
                </ul>
                
                <div id="search-input-nav">
                    <label for="nav-search">
                        <span class="fa fa-solid fa-magnifying-glass " aria-hidden="true"></span>
                    </label>

                    
                    
                    <input id="nav-search" name="query" type="text" aria-label="Find crate by search query" tabindex="-1"
                        placeholder="Find crate"
                        >
                </div>
            </form>
        </div>
    </div>
</div><div class="rustdoc mod crate container-rustdoc" id="rustdoc_body_wrapper" tabindex="-1"><script async src="/-/static/menu.js?0-0-0-9439cafc1e2ea4a64558a489c81351cd959d6077-2026-09-22"></script>
<script async src="/-/static/index.js?0-0-0-9439cafc1e2ea4a64558a489c81351cd959d6077-2026-09-22"></script>

<iframe src="/-/storage-change-detection.html" width="0" height="0" style="display: none"></iframe><a class="skip-main-content" href="#main-content">Skip to main content</a><!--[if lte IE 11]><div class="warning">This old browser is unsupported and will most likely display funky things.</div><![endif]--><rustdoc-topbar><h2><a href="#">Crate restate_sdk</a></h2></rustdoc-topbar><nav class="sidebar"><div class="sidebar-crate"><h2><a href="../restate_sdk/index.html">restate_<wbr>sdk</a><span class="version">0.12.1</span></h2></div><div class="sidebar-elems"><ul class="block"><li><a id="all-types" href="all.html">All Items</a></li></ul><section id="rustdoc-toc"><h3><a href="#">Sections</a></h3><ul class="block top-toc"><li><a href="#restate-rust-sdk" title="Restate Rust SDK">Restate Rust SDK</a><ul><li><a href="#new-to-restate" title="New to Restate?">New to Restate?</a></li></ul></li><li><a href="#features" title="Features">Features</a></li><li><a href="#sdk-overview" title="SDK Overview">SDK Overview</a><ul><li><a href="#services" title="Services">Services</a></li><li><a href="#virtual-objects" title="Virtual Objects">Virtual Objects</a></li><li><a href="#workflows" title="Workflows">Workflows</a></li></ul></li></ul><h3><a href="#reexports">Crate Items</a></h3><ul class="block"><li><a href="#reexports" title="Re-exports">Re-exports</a></li><li><a href="#modules" title="Modules">Modules</a></li><li><a href="#macros" title="Macros">Macros</a></li><li><a href="#attributes" title="Attribute Macros">Attribute Macros</a></li></ul></section><div id="rustdoc-modnav"></div></div></nav><div class="sidebar-resizer" title="Drag to resize sidebar"></div><main><div class="width-limiter"><section id="main-content" class="content" tabindex="-1"><div class="main-heading"><h1>Crate <span>restate_<wbr>sdk</span>&nbsp;<button id="copy-path" title="Copy item path to clipboard">Copy item path</button></h1><rustdoc-toolbar></rustdoc-toolbar><span class="sub-heading"><a class="src" href="../src/restate_sdk/lib.rs.html#1-600">Source</a> </span></div><details class="toggle top-doc" open><summary class="hideme"><span>Expand description</span></summary><div class="docblock"><h2 id="restate-rust-sdk"><a class="doc-anchor" href="#restate-rust-sdk">§</a>Restate Rust SDK</h2>
<p><a href="https://restate.dev/">Restate</a> is a system for easily building resilient applications.
This crate is the Restate SDK for writing Restate services using Rust.</p>
<h3 id="new-to-restate"><a class="doc-anchor" href="#new-to-restate">§</a>New to Restate?</h3>
<p>If you are new to Restate, we recommend the following resources:</p>
<ul>
<li><a href="https://docs.restate.dev/concepts/durable_building_blocks">Learn about the concepts of Restate</a></li>
<li>Use cases:
<ul>
<li><a href="https://docs.restate.dev/use-cases/workflows">Workflows</a></li>
<li><a href="https://docs.restate.dev/use-cases/microservice-orchestration">Microservice orchestration</a></li>
<li><a href="https://docs.restate.dev/use-cases/event-processing">Event processing</a></li>
<li><a href="https://docs.restate.dev/use-cases/async-tasks">Async tasks</a></li>
</ul>
</li>
<li><a href="https://docs.restate.dev/get_started/quickstart?sdk=rust">Quickstart</a></li>
<li><a href="https://docs.restate.dev/get_started/tour/?sdk=rust">Do the Tour of Restate to try out the APIs</a></li>
</ul>
<h2 id="features"><a class="doc-anchor" href="#features">§</a>Features</h2>
<p>Have a look at the following SDK capabilities:</p>
<ul>
<li><a href="#sdk-overview">SDK Overview</a>: Overview of the SDK and how to implement services, virtual objects, and workflows.</li>
<li><a href="configuration/index.html" title="mod restate_sdk::configuration">Configuration</a>: Configure services, objects, workflows and their handlers — timeouts, retention, private-ness and the invocation retry policy — via attribute arguments.</li>
<li><a href="context/trait.ContextClient.html" title="trait restate_sdk::context::ContextClient">Service Communication</a>: Durable RPC and messaging between services (optionally with a delay).</li>
<li><a href="ingress/index.html" title="mod restate_sdk::ingress">Ingress Client</a>: Invoke handlers from external applications with generated,
typed clients or a custom buffered HTTP transport.</li>
<li><a href="context/trait.ContextSideEffects.html" title="trait restate_sdk::context::ContextSideEffects">Journaling Results</a>: Persist results in Restate’s log to avoid re-execution on retries</li>
<li>State: <a href="context/trait.ContextReadState.html" title="trait restate_sdk::context::ContextReadState">read</a> and <a href="context/trait.ContextWriteState.html" title="trait restate_sdk::context::ContextWriteState">write</a>: Store and retrieve state in Restate’s key-value store</li>
<li><a href="context/trait.ContextTimers.html" title="trait restate_sdk::context::ContextTimers">Scheduling &amp; Timers</a>: Let a handler pause for a certain amount of time. Restate durably tracks the timer across failures.</li>
<li><a href="context/trait.ContextAwakeables.html" title="trait restate_sdk::context::ContextAwakeables">Awakeables</a>: Durable Futures to wait for events and the completion of external tasks.</li>
<li><a href="context/trait.ContextSignals.html" title="trait restate_sdk::context::ContextSignals">Signals</a>: Named durable promises scoped to an invocation, for communication between invocations.</li>
<li><a href="errors/index.html" title="mod restate_sdk::errors">Error Handling</a>: Restate retries failures infinitely. Use <code>TerminalError</code> to stop retries.</li>
<li><a href="serde/index.html" title="mod restate_sdk::serde">Serialization</a>: The SDK serializes results to send them to the Server. Includes <a href="serde/trait.PayloadMetadata.html" title="trait restate_sdk::serde::PayloadMetadata">Schema Generation and payload metadata</a> for documentation &amp; discovery.</li>
<li><a href="http_server/index.html" title="mod restate_sdk::http_server">Serving</a>: Start an HTTP server to expose services.</li>
</ul>
<h2 id="sdk-overview"><a class="doc-anchor" href="#sdk-overview">§</a>SDK Overview</h2>
<p>The Restate Rust SDK lets you implement durable handlers. Handlers can be part of three types of services:</p>
<ul>
<li><a href="https://docs.restate.dev/concepts/services/#services-1">Services</a>: a collection of durable handlers</li>
<li><a href="https://docs.restate.dev/concepts/services/#virtual-objects">Virtual Objects</a>: an object consists of a collection of durable handlers and isolated K/V state. Virtual Objects are useful for modeling stateful entities, where at most one handler can run at a time per object.</li>
<li><a href="https://docs.restate.dev/concepts/services/#workflows">Workflows</a>: Workflows have a <code>run</code> handler that executes exactly once per workflow instance, and executes a set of steps durably. Workflows can have other handlers that can be called multiple times and interact with the workflow.</li>
</ul>
<h3 id="services"><a class="doc-anchor" href="#services">§</a>Services</h3>
<p><a href="https://docs.restate.dev/concepts/services/#services-1">Services</a> and their handlers are defined as follows:</p>

<div class="example-wrap"><pre class="rust rust-example-rendered"><code><span class="comment">// The prelude contains all the imports you need to get started
</span><span class="kw">use </span>restate_sdk::prelude::<span class="kw-2">*</span>;

<span class="comment">// Define the service by annotating an impl block
</span><span class="kw">struct </span>MyService;

<span class="attr">#[restate_sdk::service]
</span><span class="kw">impl </span>MyService {
    <span class="attr">#[handler]
    </span><span class="kw">async fn </span>my_handler(<span class="kw-2">&amp;</span><span class="self">self</span>, ctx: Context&lt;<span class="lifetime">'_</span>&gt;, greeting: String) -&gt; <span class="prelude-ty">Result</span>&lt;String, HandlerError&gt; {
        <span class="prelude-val">Ok</span>(<span class="macro">format!</span>(<span class="string">"{greeting}!"</span>))
    }
}

<span class="comment">// Start the HTTP server to expose services
</span><span class="attr">#[tokio::main]
</span><span class="kw">async fn </span>main() {
    <span class="attr">#[cfg(feature = <span class="string">"http_server"</span>)]
    </span>{
        HttpServer::new(Endpoint::builder().bind(MyService).build())
            .listen_and_serve(<span class="string">"0.0.0.0:9080"</span>.parse().unwrap())
            .<span class="kw">await</span>;
    }
}</code></pre></div>
<ul>
<li>Define a service by putting the <a href="attr.service.html" title="attr restate_sdk::service"><code>#[restate_sdk::service]</code> macro</a> on an <code>impl</code> block of a <code>struct</code>, and annotate each handler with <a href="attr.handler.html" title="attr restate_sdk::handler"><code>#[handler]</code></a>.
<ul>
<li>Handlers take <code>&amp;self</code>, a <a href="context/struct.Context.html" title="struct restate_sdk::context::Context"><code>Context</code></a>, and optionally one input parameter, and return a <a href="https://doc.rust-lang.org/nightly/core/result/enum.Result.html" title="enum core::result::Result"><code>Result</code></a>.</li>
<li>The type of the input parameter of the handler needs to implement <a href="serde/trait.Deserialize.html" title="trait restate_sdk::serde::Deserialize"><code>Serialize</code></a> and <a href="serde/trait.Deserialize.html" title="trait restate_sdk::serde::Deserialize"><code>Deserialize</code></a>. See <a href="serde/index.html" title="mod restate_sdk::serde"><code>crate::serde</code></a>.</li>
<li>The Result contains the return value or a <a href="errors/struct.HandlerError.html" title="struct restate_sdk::errors::HandlerError"><code>HandlerError</code></a>, which can be a <a href="errors/struct.TerminalError.html" title="struct restate_sdk::errors::TerminalError"><code>TerminalError</code></a> or any other Rust’s <a href="https://doc.rust-lang.org/nightly/core/error/trait.Error.html" title="trait core::error::Error"><code>std::error::Error</code></a>.</li>
<li>The service handler can now be called at <code>&lt;RESTATE_INGRESS_URL&gt;/restate/call/MyService/my_handler</code>. You can optionally override the handler name used via <code>#[handler(name = "myHandler")]</code>, and the service name via <code>#[restate_sdk::service(name = "MyService")]</code>. More details on handler invocations can be found in the <a href="https://docs.restate.dev/invoke/http">docs</a>.</li>
</ul>
</li>
<li>Store dependencies (e.g. clients, config) as fields on the <code>struct</code> and access them via <code>&amp;self</code>. The struct is shared (behind an <a href="https://doc.rust-lang.org/nightly/alloc/sync/struct.Arc.html" title="struct alloc::sync::Arc"><code>Arc</code></a>) across all concurrent invocations, so use interior mutability (e.g. a <a href="https://doc.rust-lang.org/nightly/std/sync/poison/mutex/struct.Mutex.html" title="struct std::sync::poison::mutex::Mutex"><code>Mutex</code></a> or atomics) for any mutable state.</li>
<li>The parameter after <code>&amp;self</code> is always a <a href="context/struct.Context.html" title="struct restate_sdk::context::Context"><code>Context</code></a> to interact with Restate.
The SDK stores the actions you do on the context in the Restate journal to make them durable.</li>
<li>Finally, create an HTTP endpoint and bind the service(s) to it — pass the value directly to <a href="endpoint/struct.Builder.html#method.bind" title="method restate_sdk::endpoint::Builder::bind"><code>bind</code></a>, no <code>.serve()</code> needed. Listen on the specified port (here 9080) for connections and requests.</li>
</ul>
<h3 id="virtual-objects"><a class="doc-anchor" href="#virtual-objects">§</a>Virtual Objects</h3>
<p><a href="https://docs.restate.dev/concepts/services/#virtual-objects">Virtual Objects</a> and their handlers are defined similarly to services, with the following differences:</p>

<div class="example-wrap"><pre class="rust rust-example-rendered"><code><span class="kw">use </span>restate_sdk::prelude::<span class="kw-2">*</span>;

<span class="kw">pub struct </span>MyVirtualObject;

<span class="attr">#[restate_sdk::object]
</span><span class="kw">impl </span>MyVirtualObject {
    <span class="attr">#[handler]
    </span><span class="kw">async fn </span>my_handler(
        <span class="kw-2">&amp;</span><span class="self">self</span>,
        ctx: ObjectContext&lt;<span class="lifetime">'_</span>&gt;,
        greeting: String,
    ) -&gt; <span class="prelude-ty">Result</span>&lt;String, HandlerError&gt; {
        <span class="prelude-val">Ok</span>(<span class="macro">format!</span>(<span class="string">"{} {}"</span>, greeting, ctx.key()))
    }

    <span class="attr">#[handler]
    </span><span class="kw">async fn </span>my_concurrent_handler(
        <span class="kw-2">&amp;</span><span class="self">self</span>,
        ctx: SharedObjectContext&lt;<span class="lifetime">'_</span>&gt;,
        greeting: String,
    ) -&gt; <span class="prelude-ty">Result</span>&lt;String, HandlerError&gt; {
        <span class="prelude-val">Ok</span>(<span class="macro">format!</span>(<span class="string">"{} {}"</span>, greeting, ctx.key()))
    }
}

<span class="attr">#[tokio::main]
</span><span class="kw">async fn </span>main() {
    <span class="attr">#[cfg(feature = <span class="string">"http_server"</span>)]
    </span>{
        HttpServer::new(Endpoint::builder().bind(MyVirtualObject).build())
            .listen_and_serve(<span class="string">"0.0.0.0:9080"</span>.parse().unwrap())
            .<span class="kw">await</span>;
    }
}</code></pre></div>
<ul>
<li>Specify that you want to create a Virtual Object by putting the <a href="attr.object.html" title="attr restate_sdk::object"><code>#[restate_sdk::object]</code> macro</a> on the <code>impl</code> block.</li>
<li>The context after <code>&amp;self</code> must be the <a href="context/struct.ObjectContext.html" title="struct restate_sdk::context::ObjectContext"><code>ObjectContext</code></a> parameter. Handlers with the <code>ObjectContext</code> parameter can write to the K/V state store. Only one handler can be active at a time per object, to ensure consistency.</li>
<li>You can retrieve the key of the object you are in via [<code>ObjectContext.key</code>].</li>
<li>If you want to have a handler that executes concurrently to the others and doesn’t have write access to the K/V state, use the <a href="context/struct.SharedObjectContext.html" title="struct restate_sdk::context::SharedObjectContext"><code>SharedObjectContext</code></a> as its context.
The shared/exclusive kind is inferred from the context type.
You can use these handlers, for example, to read K/V state and expose it to the outside world, or to interact with the blocking handler and resolve awakeables etc.</li>
</ul>
<h3 id="workflows"><a class="doc-anchor" href="#workflows">§</a>Workflows</h3>
<p><a href="https://docs.restate.dev/concepts/services/#workflows">Workflows</a> are a special type of Virtual Objects, their definition is similar but with the following differences:</p>

<div class="example-wrap"><pre class="rust rust-example-rendered"><code><span class="kw">use </span>restate_sdk::prelude::<span class="kw-2">*</span>;

<span class="kw">pub struct </span>MyWorkflow;

<span class="attr">#[restate_sdk::workflow]
</span><span class="kw">impl </span>MyWorkflow {
    <span class="attr">#[handler]
    </span><span class="kw">async fn </span>run(<span class="kw-2">&amp;</span><span class="self">self</span>, ctx: WorkflowContext&lt;<span class="lifetime">'_</span>&gt;, req: String) -&gt; <span class="prelude-ty">Result</span>&lt;String, HandlerError&gt; {
        <span class="comment">// implement workflow logic here

        </span><span class="prelude-val">Ok</span>(String::from(<span class="string">"success"</span>))
    }

    <span class="attr">#[handler]
    </span><span class="kw">async fn </span>interact_with_workflow(<span class="kw-2">&amp;</span><span class="self">self</span>, ctx: SharedWorkflowContext&lt;<span class="lifetime">'_</span>&gt;) -&gt; <span class="prelude-ty">Result</span>&lt;(), HandlerError&gt; {
        <span class="comment">// implement interaction logic here
        // e.g. resolve a promise that the workflow is waiting on

        </span><span class="prelude-val">Ok</span>(())
    }
}

<span class="attr">#[tokio::main]
</span><span class="kw">async fn </span>main() {
    <span class="attr">#[cfg(feature = <span class="string">"http_server"</span>)]
    </span>{
        HttpServer::new(Endpoint::builder().bind(MyWorkflow).build())
            .listen_and_serve(<span class="string">"0.0.0.0:9080"</span>.parse().unwrap())
            .<span class="kw">await</span>;
    }
}</code></pre></div>
<ul>
<li>Specify that you want to create a Workflow by putting the <a href="attr.workflow.html" title="attr restate_sdk::workflow"><code>#[restate_sdk::workflow]</code> macro</a> on the <code>impl</code> block.</li>
<li>The workflow needs to have a <code>run</code> handler.</li>
<li>The context of the <code>run</code> handler must be the <a href="context/struct.WorkflowContext.html" title="struct restate_sdk::context::WorkflowContext"><code>WorkflowContext</code></a> parameter.
The <code>WorkflowContext</code> parameter is used to interact with Restate.
The <code>run</code> handler executes exactly once per workflow instance.</li>
<li>The other handlers of the workflow are used to interact with the workflow: either query it, or signal it.
They use the <a href="context/struct.SharedWorkflowContext.html" title="struct restate_sdk::context::SharedWorkflowContext"><code>SharedWorkflowContext</code></a> to interact with the SDK.
These handlers can run concurrently with the run handler and can still be called after the run handler has finished.</li>
<li>Have a look at the <a href="attr.workflow.html" title="attr restate_sdk::workflow">workflow docs</a> to learn more.</li>
</ul>
<p>Learn more about each service type here:</p>
<ul>
<li><a href="attr.service.html" title="attr restate_sdk::service">Service</a></li>
<li><a href="attr.object.html" title="attr restate_sdk::object">Virtual Object</a></li>
<li><a href="attr.workflow.html" title="attr restate_sdk::workflow">Workflow</a></li>
</ul>
<h4 id="calling-handlers-through-ingress"><a class="doc-anchor" href="#calling-handlers-through-ingress">§</a>Calling handlers through ingress</h4>
<p>The SDK generates a typed <code>&lt;Type&gt;IngressClient</code> for every impl-block service, virtual object,
and workflow. Ingress clients invoke handlers from outside a Restate service and require Restate
1.7 or newer.</p>
<p>Enable the <code>reqwest-client</code> feature to use the built-in reqwest client:</p>

<div class="example-wrap"><pre class="rust rust-example-rendered"><code><span class="kw">use </span>restate_sdk::ingress::ReqwestClient;

<span class="kw">let </span>client = ReqwestClient::connect(<span class="string">"http://localhost:8080"</span>.parse()<span class="question-mark">?</span>)<span class="question-mark">?</span>;
<span class="kw">let </span>greeter = GreeterIngressClient::from_client(client);
<span class="kw">let </span>greeting = greeter
    .greet(<span class="string">"Ada"</span>.to_owned())
    .call()
    .<span class="kw">await</span><span class="question-mark">?
    </span>.into_body()<span class="question-mark">?</span>;

<span class="macro">println!</span>(<span class="string">"{greeting}"</span>);</code></pre></div>
<p>To use another HTTP client, implement <a href="ingress/trait.RequestExecutor.html" title="trait restate_sdk::ingress::RequestExecutor"><code>ingress::RequestExecutor</code></a> and pass it to
<a href="ingress/struct.Client.html#method.new" title="associated function restate_sdk::ingress::Client::new"><code>ingress::Client::new</code></a>.</p>
<h4 id="logging"><a class="doc-anchor" href="#logging">§</a>Logging</h4>
<p>This crate uses the <a href="https://docs.rs/tracing/0.1.41/x86_64-unknown-linux-gnu/tracing/index.html" title="mod tracing">tracing crate</a> to emit logs, so you’ll need to configure a tracing subscriber to get logs. For example, to configure console logging using <code>tracing_subscriber::fmt</code>:</p>

<div class="example-wrap"><pre class="rust rust-example-rendered"><code><span class="attr">#[tokio::main]
</span><span class="kw">async fn </span>main() {
    <span class="doccomment">//! To enable logging
    </span>tracing_subscriber::fmt::init();

    <span class="comment">// Start http server etc...
</span>}</code></pre></div>
<p>You can filter logs <em>when a handler is being replayed</em> configuring the <a href="filter/struct.ReplayAwareFilter.html" title="struct restate_sdk::filter::ReplayAwareFilter">filter::ReplayAwareFilter</a>.</p>
<p>For more information about tracing and logging, have a look at the <a href="https://docs.rs/tracing-subscriber/latest/tracing_subscriber/fmt/index.html#filtering-events-with-environment-variables">tracing subscriber doc</a>.</p>
<p>Next, have a look at the other <a href="#features">SDK features</a>.</p>
</div></details><h2 id="reexports" class="section-header">Re-exports<a href="#reexports" class="anchor">§</a></h2><dl class="item-table reexports"><dt id="reexport.rand"><code>pub use <a class="mod" href="https://docs.rs/rand/0.8.5/x86_64-unknown-linux-gnu/rand/index.html" title="mod rand">rand</a>;</code></dt><dt id="reexport.uuid"><code>pub use <a class="mod" href="https://docs.rs/uuid/1.23.0/x86_64-unknown-linux-gnu/uuid/index.html" title="mod uuid">uuid</a>;</code></dt></dl><h2 id="modules" class="section-header">Modules<a href="#modules" class="anchor">§</a></h2><dl class="item-table"><dt><a class="mod" href="configuration/index.html" title="mod restate_sdk::configuration">configuration</a></dt><dd>Configuring services and handlers</dd><dt><a class="mod" href="context/index.html" title="mod restate_sdk::context">context</a></dt><dd>Types exposing Restate functionalities to service handlers.</dd><dt><a class="mod" href="discovery/index.html" title="mod restate_sdk::discovery">discovery</a></dt><dd>This module contains the generated data structures from the <a href="https://github.com/restatedev/service-protocol/blob/main/endpoint_manifest_schema.json">service protocol manifest schema</a>.</dd><dt><a class="mod" href="endpoint/index.html" title="mod restate_sdk::endpoint">endpoint</a></dt><dt><a class="mod" href="errors/index.html" title="mod restate_sdk::errors">errors</a></dt><dd>Error Handling</dd><dt><a class="mod" href="filter/index.html" title="mod restate_sdk::filter">filter</a></dt><dd>Replay aware tracing filter.</dd><dt><a class="mod" href="http_server/index.html" title="mod restate_sdk::http_server">http_<wbr>server</a></dt><dd>Serving</dd><dt><a class="mod" href="hyper/index.html" title="mod restate_sdk::hyper">hyper</a></dt><dd>Hyper integration.</dd><dt><a class="mod" href="ingress/index.html" title="mod restate_sdk::ingress">ingress</a></dt><dd>Ingress client</dd><dt><a class="mod" href="prelude/index.html" title="mod restate_sdk::prelude">prelude</a></dt><dd>Prelude contains all the useful imports you need to get started with Restate.</dd><dt><a class="mod" href="serde/index.html" title="mod restate_sdk::serde">serde</a></dt><dd>Serialization</dd><dt><a class="mod" href="service/index.html" title="mod restate_sdk::service">service</a></dt></dl><h2 id="macros" class="section-header">Macros<a href="#macros" class="anchor">§</a></h2><dl class="item-table"><dt><a class="macro" href="macro.select.html" title="macro restate_sdk::select">select</a></dt><dd>Select macro, alike tokio::select:</dd></dl><h2 id="attributes" class="section-header">Attribute Macros<a href="#attributes" class="anchor">§</a></h2><dl class="item-table"><dt><a class="attr" href="attr.handler.html" title="attr restate_sdk::handler">handler</a></dt><dd>Marks a method inside a <code>#[restate_sdk::service]</code>/<code>#[object]</code>/<code>#[workflow]</code> impl block as a
Restate handler.</dd><dt><a class="attr" href="attr.object.html" title="attr restate_sdk::object">object</a></dt><dd>Entry-point macro to define a Restate <a href="https://docs.restate.dev/concepts/services#virtual-objects">Virtual object</a>.</dd><dt><a class="attr" href="attr.service.html" title="attr restate_sdk::service">service</a></dt><dd>Entry-point macro to define a Restate <a href="https://docs.restate.dev/concepts/services#services-1">Service</a>.</dd><dt><a class="attr" href="attr.workflow.html" title="attr restate_sdk::workflow">workflow</a></dt><dd>Workflows</dd></dl><script type="text/json" id="notable-traits-data">{"&[u8]":"<h3>Notable traits for <code>&amp;[<a class=\"primitive\" href=\"https://doc.rust-lang.org/nightly/std/primitive.u8.html\">u8</a>]</code></h3><pre><code><div class=\"where\">impl <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/alloc/io/read/trait.Read.html\" title=\"trait alloc::io::read::Read\">Read</a> for &amp;[<a class=\"primitive\" href=\"https://doc.rust-lang.org/nightly/std/primitive.u8.html\">u8</a>]</div>","&mut Vec<u8>":"<h3>Notable traits for <code><a class=\"struct\" href=\"https://doc.rust-lang.org/nightly/alloc/vec/struct.Vec.html\" title=\"struct alloc::vec::Vec\">Vec</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/nightly/std/primitive.u8.html\">u8</a>, A&gt;</code></h3><pre><code><div class=\"where\">impl&lt;A&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/io/write/trait.Write.html\" title=\"trait core::io::write::Write\">Write</a> for <a class=\"struct\" href=\"https://doc.rust-lang.org/nightly/alloc/vec/struct.Vec.html\" title=\"struct alloc::vec::Vec\">Vec</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/nightly/std/primitive.u8.html\">u8</a>, A&gt;<div class=\"where\">where\n    A: <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/alloc/trait.Allocator.html\" title=\"trait core::alloc::Allocator\">Allocator</a>,</div></div>","Drain<'_, T>":"<h3>Notable traits for <code><a class=\"struct\" href=\"https://docs.rs/http/1.4.0/x86_64-unknown-linux-gnu/http/header/map/struct.Drain.html\" title=\"struct http::header::map::Drain\">Drain</a>&lt;'a, T&gt;</code></h3><pre><code><div class=\"where\">impl&lt;'a, T&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/iter/traits/iterator/trait.Iterator.html\" title=\"trait core::iter::traits::iterator::Iterator\">Iterator</a> for <a class=\"struct\" href=\"https://docs.rs/http/1.4.0/x86_64-unknown-linux-gnu/http/header/map/struct.Drain.html\" title=\"struct http::header::map::Drain\">Drain</a>&lt;'a, T&gt;</div><div class=\"where\">    type <a href=\"https://doc.rust-lang.org/nightly/core/iter/traits/iterator/trait.Iterator.html#associatedtype.Item\" class=\"associatedtype\">Item</a> = (<a class=\"enum\" href=\"https://doc.rust-lang.org/nightly/core/option/enum.Option.html\" title=\"enum core::option::Option\">Option</a>&lt;<a class=\"struct\" href=\"https://docs.rs/http/1.4.0/x86_64-unknown-linux-gnu/http/header/name/struct.HeaderName.html\" title=\"struct http::header::name::HeaderName\">HeaderName</a>&gt;, T);</div>","Drain<'_>":"<h3>Notable traits for <code><a class=\"struct\" href=\"https://doc.rust-lang.org/nightly/alloc/string/struct.Drain.html\" title=\"struct alloc::string::Drain\">Drain</a>&lt;'_&gt;</code></h3><pre><code><div class=\"where\">impl <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/iter/traits/iterator/trait.Iterator.html\" title=\"trait core::iter::traits::iterator::Iterator\">Iterator</a> for <a class=\"struct\" href=\"https://doc.rust-lang.org/nightly/alloc/string/struct.Drain.html\" title=\"struct alloc::string::Drain\">Drain</a>&lt;'_&gt;</div><div class=\"where\">    type <a href=\"https://doc.rust-lang.org/nightly/core/iter/traits/iterator/trait.Iterator.html#associatedtype.Item\" class=\"associatedtype\">Item</a> = <a class=\"primitive\" href=\"https://doc.rust-lang.org/nightly/std/primitive.char.html\">char</a>;</div>","IntoChars":"<h3>Notable traits for <code><a class=\"struct\" href=\"https://doc.rust-lang.org/nightly/alloc/string/struct.IntoChars.html\" title=\"struct alloc::string::IntoChars\">IntoChars</a></code></h3><pre><code><div class=\"where\">impl <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/iter/traits/iterator/trait.Iterator.html\" title=\"trait core::iter::traits::iterator::Iterator\">Iterator</a> for <a class=\"struct\" href=\"https://doc.rust-lang.org/nightly/alloc/string/struct.IntoChars.html\" title=\"struct alloc::string::IntoChars\">IntoChars</a></div><div class=\"where\">    type <a href=\"https://doc.rust-lang.org/nightly/core/iter/traits/iterator/trait.Iterator.html#associatedtype.Item\" class=\"associatedtype\">Item</a> = <a class=\"primitive\" href=\"https://doc.rust-lang.org/nightly/std/primitive.char.html\">char</a>;</div>","IntoIter<T>":"<h3>Notable traits for <code><a class=\"struct\" href=\"https://doc.rust-lang.org/nightly/core/result/struct.IntoIter.html\" title=\"struct core::result::IntoIter\">IntoIter</a>&lt;T&gt;</code></h3><pre><code><div class=\"where\">impl&lt;T&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/iter/traits/iterator/trait.Iterator.html\" title=\"trait core::iter::traits::iterator::Iterator\">Iterator</a> for <a class=\"struct\" href=\"https://doc.rust-lang.org/nightly/core/result/struct.IntoIter.html\" title=\"struct core::result::IntoIter\">IntoIter</a>&lt;T&gt;</div><div class=\"where\">    type <a href=\"https://doc.rust-lang.org/nightly/core/iter/traits/iterator/trait.Iterator.html#associatedtype.Item\" class=\"associatedtype\">Item</a> = T;</div>","Iter<'_, T>":"<h3>Notable traits for <code><a class=\"struct\" href=\"https://doc.rust-lang.org/nightly/core/result/struct.Iter.html\" title=\"struct core::result::Iter\">Iter</a>&lt;'a, T&gt;</code></h3><pre><code><div class=\"where\">impl&lt;'a, T&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/iter/traits/iterator/trait.Iterator.html\" title=\"trait core::iter::traits::iterator::Iterator\">Iterator</a> for <a class=\"struct\" href=\"https://doc.rust-lang.org/nightly/core/result/struct.Iter.html\" title=\"struct core::result::Iter\">Iter</a>&lt;'a, T&gt;</div><div class=\"where\">    type <a href=\"https://doc.rust-lang.org/nightly/core/iter/traits/iterator/trait.Iterator.html#associatedtype.Item\" class=\"associatedtype\">Item</a> = <a class=\"primitive\" href=\"https://doc.rust-lang.org/nightly/std/primitive.reference.html\">&amp;'a T</a>;</div>","IterMut<'_, T>":"<h3>Notable traits for <code><a class=\"struct\" href=\"https://doc.rust-lang.org/nightly/core/result/struct.IterMut.html\" title=\"struct core::result::IterMut\">IterMut</a>&lt;'a, T&gt;</code></h3><pre><code><div class=\"where\">impl&lt;'a, T&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/iter/traits/iterator/trait.Iterator.html\" title=\"trait core::iter::traits::iterator::Iterator\">Iterator</a> for <a class=\"struct\" href=\"https://doc.rust-lang.org/nightly/core/result/struct.IterMut.html\" title=\"struct core::result::IterMut\">IterMut</a>&lt;'a, T&gt;</div><div class=\"where\">    type <a href=\"https://doc.rust-lang.org/nightly/core/iter/traits/iterator/trait.Iterator.html#associatedtype.Item\" class=\"associatedtype\">Item</a> = <a class=\"primitive\" href=\"https://doc.rust-lang.org/nightly/std/primitive.reference.html\">&amp;'a mut T</a>;</div>","Keys<'_, T>":"<h3>Notable traits for <code><a class=\"struct\" href=\"https://docs.rs/http/1.4.0/x86_64-unknown-linux-gnu/http/header/map/struct.Keys.html\" title=\"struct http::header::map::Keys\">Keys</a>&lt;'a, T&gt;</code></h3><pre><code><div class=\"where\">impl&lt;'a, T&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/iter/traits/iterator/trait.Iterator.html\" title=\"trait core::iter::traits::iterator::Iterator\">Iterator</a> for <a class=\"struct\" href=\"https://docs.rs/http/1.4.0/x86_64-unknown-linux-gnu/http/header/map/struct.Keys.html\" title=\"struct http::header::map::Keys\">Keys</a>&lt;'a, T&gt;</div><div class=\"where\">    type <a href=\"https://doc.rust-lang.org/nightly/core/iter/traits/iterator/trait.Iterator.html#associatedtype.Item\" class=\"associatedtype\">Item</a> = &amp;'a <a class=\"struct\" href=\"https://docs.rs/http/1.4.0/x86_64-unknown-linux-gnu/http/header/name/struct.HeaderName.html\" title=\"struct http::header::name::HeaderName\">HeaderName</a>;</div>","Values<'_, T>":"<h3>Notable traits for <code><a class=\"struct\" href=\"https://docs.rs/http/1.4.0/x86_64-unknown-linux-gnu/http/header/map/struct.Values.html\" title=\"struct http::header::map::Values\">Values</a>&lt;'a, T&gt;</code></h3><pre><code><div class=\"where\">impl&lt;'a, T&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/iter/traits/iterator/trait.Iterator.html\" title=\"trait core::iter::traits::iterator::Iterator\">Iterator</a> for <a class=\"struct\" href=\"https://docs.rs/http/1.4.0/x86_64-unknown-linux-gnu/http/header/map/struct.Values.html\" title=\"struct http::header::map::Values\">Values</a>&lt;'a, T&gt;</div><div class=\"where\">    type <a href=\"https://doc.rust-lang.org/nightly/core/iter/traits/iterator/trait.Iterator.html#associatedtype.Item\" class=\"associatedtype\">Item</a> = <a class=\"primitive\" href=\"https://doc.rust-lang.org/nightly/std/primitive.reference.html\">&amp;'a T</a>;</div>","ValuesMut<'_, T>":"<h3>Notable traits for <code><a class=\"struct\" href=\"https://docs.rs/http/1.4.0/x86_64-unknown-linux-gnu/http/header/map/struct.ValuesMut.html\" title=\"struct http::header::map::ValuesMut\">ValuesMut</a>&lt;'a, T&gt;</code></h3><pre><code><div class=\"where\">impl&lt;'a, T&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/iter/traits/iterator/trait.Iterator.html\" title=\"trait core::iter::traits::iterator::Iterator\">Iterator</a> for <a class=\"struct\" href=\"https://docs.rs/http/1.4.0/x86_64-unknown-linux-gnu/http/header/map/struct.ValuesMut.html\" title=\"struct http::header::map::ValuesMut\">ValuesMut</a>&lt;'a, T&gt;</div><div class=\"where\">    type <a href=\"https://doc.rust-lang.org/nightly/core/iter/traits/iterator/trait.Iterator.html#associatedtype.Item\" class=\"associatedtype\">Item</a> = <a class=\"primitive\" href=\"https://doc.rust-lang.org/nightly/std/primitive.reference.html\">&amp;'a mut T</a>;</div>","Vec<u8>":"<h3>Notable traits for <code><a class=\"struct\" href=\"https://doc.rust-lang.org/nightly/alloc/vec/struct.Vec.html\" title=\"struct alloc::vec::Vec\">Vec</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/nightly/std/primitive.u8.html\">u8</a>, A&gt;</code></h3><pre><code><div class=\"where\">impl&lt;A&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/io/write/trait.Write.html\" title=\"trait core::io::write::Write\">Write</a> for <a class=\"struct\" href=\"https://doc.rust-lang.org/nightly/alloc/vec/struct.Vec.html\" title=\"struct alloc::vec::Vec\">Vec</a>&lt;<a class=\"primitive\" href=\"https://doc.rust-lang.org/nightly/std/primitive.u8.html\">u8</a>, A&gt;<div class=\"where\">where\n    A: <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/alloc/trait.Allocator.html\" title=\"trait core::alloc::Allocator\">Allocator</a>,</div></div>"}</script></section></div></main></div></body></html>