# Heimspiel

Heimspiel gathers the games a person owns from several places into one library and presents it
in frontends such as ES-DE. It installs, configures and connects existing projects instead of
replacing them.

## Library

**Library**:
All Entries heimspiel knows about, across every Source.
_Avoid_: collection, catalog, database

**Entry**:
One playable game in the Library, as delivered by exactly one Source. The same game bought on two
stores is two Entries.
_Avoid_: game (when the record is meant), item, title, ROM

**Source**:
A place Entries come from: Steam, Heroic, manual PC installs, a RomM server or a local folder.
_Avoid_: provider, importer, integration, store

**Output**:
A frontend or launcher that heimspiel writes the Library into so it can be browsed and played
there, such as ES-DE, and later Steam or Playnite.
_Avoid_: target, exporter, frontend (when the written result is meant)

**Platform**:
The console or computer a game was made for, such as Nintendo 64 or Game Boy Advance.
_Avoid_: system (ES-DE's word for its menu of a Platform), console, core

**Placeholder**:
A stand-in that represents an Entry in an Output while the Entry's file is not on this machine yet.
_Avoid_: stub, dummy, shortcut

## Ports

**Port**:
A native PC build of a console game, made by recompiling or decompiling the original, that only
runs with data from an Original. Zelda64Recomp, Ship of Harkinian and SpartanPort are Ports.
_Avoid_: recomp, decomp, PC port (as separate concepts; they are kinds of Port)

**Original**:
The copy of a console game the user owns and a Port needs, such as a ROM, a disc image or a Dump.
_Avoid_: base ROM, source game, game files

**Dump**:
An Original in the form of an extracted, decrypted folder of game files, made from the user's own
console or disc.
_Avoid_: rip, extract, backup

**Catalog**:
The list of known Ports with what each one needs, gathered from public directories of
recompilation projects.
_Avoid_: index, database, Library (the Library holds Entries, not known Ports)

**Manifest**:
heimspiel's description of one Port: where it is released, which Originals it accepts and how its
Original is provisioned.
_Avoid_: recipe, config, spec

**Provisioning**:
Putting an Original where a Port expects it, in the form it expects, so that the Port starts without
asking the user for anything.
_Avoid_: setup, install, import (installing the Port itself is not Provisioning)
