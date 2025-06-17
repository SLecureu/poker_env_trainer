## ACTIVATE THE VIRTUAL ENVIRONMENT

Recommended: Run everything inside a virtual environment:

```bash
python3 -m venv .venv
source .venv/bin/activate
pip install -r requirements.txt
```

```bash
source .venv/bin/activate
```

## ADDING / REMOVING A PACKAGE

If you want to modify the packages, here’s how to do it:

Add or remove the package in requirements.in if it’s not part of the standard library.

Then run:

```bash
pip-compile requirements.in
```

This may take several minutes. It will generate a requirements.txt file containing all dependencies and packages.

## INSTALLING / UNINSTALLING PACKAGES / DEPENDENCIES

Here are the commands to install or uninstall all packages and dependencies:

-Uninstall everything

```bash
pip freeze | grep -v '@' | xargs pip uninstall -y
```

-Install everything

```bash
pip install -r requirements.txt
```

## COMPILE THE RUST MODULE

```bash
maturin develop
```

Note: The library "rs_poker" uses Rust nightly, so you need to run:

```bash
rustup override set nightly
```

Run this if Rust nightly is not installed:

```bash
rustup toolchain install nightly
```
