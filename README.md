## ACTIVER L'ENVIRONNEMENT VIRTUEL

Conseillé: faire tourner le tout dans un environement virtuel:

```bash
python3 -m venv .venv
source .venv/bin/activate
pip install -r requirements.txt
```

```bash
source .fishking/bin/activate
```

## RAJOUTER / RETIRER UN PACKAGE

Si on veut changer des packages voici comment faire:

ajouter / retirer à requirements.in le package si il ne fait pas partie de la bibliothèque standard

exécuter:

```bash
pip-compile requirements.in
```

Cela peut prendre plusieurs minutes. Cela créra une fichier requirements.txt avec toutes les dépendances et les packages.

## INSTALLER / DÉSINSTALLER LES PACKAGES / DÉPENDANCES

Voici les commandes pour installer / désintaller tous les packages et dépendances:

-Tout désinstaller

```bash
pip freeze | grep -v '@' | xargs pip uninstall -y
```

-   TOUT INSTALLER

```bash
pip install -r requirements.txt
```

## COMPILER LE MODULE RUST

```bash
maturin develop
```

Attention, la bibliothèque utiliser "rs_poker" utilise rust nighty, il faut run:

```bash
rustup override set nightly
```

Run ceci si rust nighty n'est pas installé:

```bash
rustup toolchain install nightly
```
