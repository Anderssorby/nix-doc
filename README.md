# nix-doc

A Nix documentation search tool. This package provides two front ends for
documentation lookup: a Nix plugin that allows access to documentation directly
from `nix repl` and a command line tool.

## Setup

```
# installs both Nix plugin and command line tool
$ nix-env -i -f https://github.com/lf-/nix-doc/archive/main.tar.gz
# or if you don't want to use nix, (only includes the command line tool)
$ cargo install nix-doc
```

### Nix Plugin

To install the Nix plugin, add this to your Nix config at
`~/.config/nix/nix.conf` after installing `nix-doc` with `nix-env`:

```
plugin-files = /home/YOURUSERNAMEHERE/.nix-profile/lib/libnix_doc_plugin.so
```

## Usage

### CLI

```
nix-doc SearchRegex [Directory]
```

Example output:

```
nixpkgs$ nix-doc callPackage
   Call the package function in the file `fn' with the required
   arguments automatically.  The function is called with the
   arguments `args', but any missing arguments are obtained from
   `autoArgs'.  This function is intended to be partially
   parameterised, e.g.,

   callPackage = callPackageWith pkgs;
   pkgs = {
   libfoo = callPackage ./foo.nix { };
   libbar = callPackage ./bar.nix { };
   };

   If the `libbar' function expects an argument named `libfoo', it is
   automatically passed as an argument.  Overrides or missing
   arguments can be supplied in `args', e.g.

   libbar = callPackage ./bar.nix {
   libfoo = null;
   enableX11 = true;
   };
callPackageWith = autoArgs: fn: args: ...
# ./lib/customisation.nix:117
─────────────────────────────────────────────
   Like callPackage, but for a function that returns an attribute
   set of derivations. The override function is added to the
   individual attributes.
callPackagesWith = autoArgs: fn: args: ...
# ./lib/customisation.nix:127
─────────────────────────────────────────────
   Similar to callPackageWith/callPackage, but without makeOverridable
callPackageWith = autoArgs: fn: args: ...
# ./pkgs/development/beam-modules/lib.nix:7
```

### Nix plugin

The Nix plugin provides three builtins:

#### `builtins.doc f`

Prints the documentation of the function `f` to the screen. Returns `null`.

#### `builtins.getDoc f`

Returns the documentation message for the function `f` as a string (exactly the
same output as `builtins.doc`, just as a string).

#### `builtins.unsafeGetLambdaPos`

A backport of [NixOS/Nix#3912](https://github.com/NixOS/nix/pull/3912). Returns
the position of a lambda, in a similar fashion to `unsafeGetAttrPos` for
attributes.

#### Sample usage:

```
» nix repl
Welcome to Nix version 2.3.7. Type :? for help.

nix-repl> n=import <nixpkgs> {}

nix-repl> builtins.doc n.lib.callPackageWith
   `overrideDerivation drv f' takes a derivation (i.e., the result
   of a call to the builtin function `derivation') and returns a new
   derivation in which the attributes of the original are overridden
   according to the function `f'.  The function `f' is called with
   the original derivation attributes.

   `overrideDerivation' allows certain "ad-hoc" customisation
   scenarios (e.g. in ~/.config/nixpkgs/config.nix).  For instance,
   if you want to "patch" the derivation returned by a package
   function in Nixpkgs to build another version than what the
   function itself provides, you can do something like this:

   mySed = overrideDerivation pkgs.gnused (oldAttrs: {
   name = "sed-4.2.2-pre";
   src = fetchurl {
   url = ftp://alpha.gnu.org/gnu/sed/sed-4.2.2-pre.tar.bz2;
   sha256 = "11nq06d131y4wmf3drm0yk502d2xc6n5qy82cg88rb9nqd2lj41k";
   };
   patches = [];
   });

   For another application, see build-support/vm, where this
   function is used to build arbitrary derivations inside a QEMU
   virtual machine.
   `makeOverridable` takes a function from attribute set to attribute set and
   injects `override` attribute which can be used to override arguments of
   the function.

   nix-repl> x = {a, b}: { result = a + b; }

   nix-repl> y = lib.makeOverridable x { a = 1; b = 2; }

   nix-repl> y
   { override = «lambda»; overrideDerivation = «lambda»; result = 3; }

   nix-repl> y.override { a = 10; }
   { override = «lambda»; overrideDerivation = «lambda»; result = 12; }

   Please refer to "Nixpkgs Contributors Guide" section
   "<pkg>.overrideDerivation" to learn about `overrideDerivation` and caveats
   related to its use.
   Call the package function in the file `fn' with the required
   arguments automatically.  The function is called with the
   arguments `args', but any missing arguments are obtained from
   `autoArgs'.  This function is intended to be partially
   parameterised, e.g.,

   callPackage = callPackageWith pkgs;
   pkgs = {
   libfoo = callPackage ./foo.nix { };
   libbar = callPackage ./bar.nix { };
   };

   If the `libbar' function expects an argument named `libfoo', it is
   automatically passed as an argument.  Overrides or missing
   arguments can be supplied in `args', e.g.

   libbar = callPackage ./bar.nix {
   libfoo = null;
   enableX11 = true;
   };
func = autoArgs: fn: args: ...
# /nix/store/nm5fxk0kzm3mlx1c22byfs4jizajwbk1-nixpkgs-20.09pre237349.f9f48250fe1/nixpkgs/lib/customisation.nix:117
null
```

## Development

This repository is set up as a Cargo workspace with the plugin and the command
line tool/library as parts.

It is not really possible to build the plugin outside a Nix shell since Nix
does not provide libraries outside the shell environment. As such, it is
suggested to use a nix shell while developing the plugin as follows:

```
$ nix-shell
[nix-shell]$ cargo build
[nix-shell]$ cargo check
[nix-shell]$ cargo test
# etc
```

## TODO

* Generate tags files/otherwise generate a database file to speed up result
  generation.
* Fix the dedent again. We are eating indents we should not be eating e.g. in
  the example above.

## Project information

Everyone is expected to follow the [code of conduct](./CODE_OF_CONDUCT.md)
while participating in this project.
