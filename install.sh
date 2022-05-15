#!/bin/bash

cargo build --release
mkdir -p ~/.local/share/pop-launcher/plugins/github
install -Dm0755 target/release/pop-launcher-plugin-github ~/.local/share/pop-launcher/plugins/github/pop-launcher-plugin-github
install -Dm644 plugin.ron ~/.local/share/pop-launcher/plugins/github/plugin.ron