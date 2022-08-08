#!/usr/bin/env python3
# MAKE.py
#   by Lut99
#
# Created:
#   09 Jun 2022, 12:20:28
# Last edited:
#   04 Aug 2022, 14:12:52
# Auto updated?
#   Yes
#
# Description:
#   Python script that implements the (more advanced) make script for the
#   Brane infrastructure.
#

from __future__ import annotations

import abc
import argparse
from cmath import e
import hashlib
import http
import json
from multiprocessing.sharedctypes import Value
import os
from sre_constants import SRE_FLAG_UNICODE
import requests
import subprocess
import sys
import tarfile
import time
import typing


##### CONSTANTS #####
# List of services in an instance
SERVICES = [ "api", "clb", "drv", "job", "log", "plr" ]

# The directory where we compile OpenSSL to
OPENSSL_DIR = "./target/openssl/$ARCH"

# The desired source files that we want to build against for OpenSSL
OPENSSL_FILES = [
    OPENSSL_DIR + "/lib/libcrypto.a", OPENSSL_DIR + "/lib/libssl.a",
    OPENSSL_DIR + "/lib/pkgconfig/libcrypto.pc", OPENSSL_DIR + "/lib/pkgconfig/libssl.pc", OPENSSL_DIR + "/lib/pkgconfig/openssl.pc",
    OPENSSL_DIR + "/include/openssl/aes.h", OPENSSL_DIR + "/include/openssl/asn1err.h", OPENSSL_DIR + "/include/openssl/asn1.h",
    OPENSSL_DIR + "/include/openssl/asn1_mac.h", OPENSSL_DIR + "/include/openssl/asn1t.h", OPENSSL_DIR + "/include/openssl/asyncerr.h",
    OPENSSL_DIR + "/include/openssl/async.h", OPENSSL_DIR + "/include/openssl/bioerr.h", OPENSSL_DIR + "/include/openssl/bio.h",
    OPENSSL_DIR + "/include/openssl/blowfish.h", OPENSSL_DIR + "/include/openssl/bnerr.h", OPENSSL_DIR + "/include/openssl/bn.h",
    OPENSSL_DIR + "/include/openssl/buffererr.h", OPENSSL_DIR + "/include/openssl/buffer.h", OPENSSL_DIR + "/include/openssl/camellia.h",
    OPENSSL_DIR + "/include/openssl/cast.h", OPENSSL_DIR + "/include/openssl/cmac.h", OPENSSL_DIR + "/include/openssl/cmserr.h",
    OPENSSL_DIR + "/include/openssl/cms.h", OPENSSL_DIR + "/include/openssl/comperr.h", OPENSSL_DIR + "/include/openssl/comp.h",
    OPENSSL_DIR + "/include/openssl/conf_api.h", OPENSSL_DIR + "/include/openssl/conferr.h", OPENSSL_DIR + "/include/openssl/conf.h",
    OPENSSL_DIR + "/include/openssl/cryptoerr.h", OPENSSL_DIR + "/include/openssl/crypto.h", OPENSSL_DIR + "/include/openssl/cterr.h",
    OPENSSL_DIR + "/include/openssl/ct.h", OPENSSL_DIR + "/include/openssl/des.h", OPENSSL_DIR + "/include/openssl/dherr.h",
    OPENSSL_DIR + "/include/openssl/dh.h", OPENSSL_DIR + "/include/openssl/dsaerr.h", OPENSSL_DIR + "/include/openssl/dsa.h",
    OPENSSL_DIR + "/include/openssl/dtls1.h", OPENSSL_DIR + "/include/openssl/ebcdic.h", OPENSSL_DIR + "/include/openssl/ecdh.h",
    OPENSSL_DIR + "/include/openssl/ecdsa.h", OPENSSL_DIR + "/include/openssl/ecerr.h", OPENSSL_DIR + "/include/openssl/ec.h",
    OPENSSL_DIR + "/include/openssl/engineerr.h", OPENSSL_DIR + "/include/openssl/engine.h", OPENSSL_DIR + "/include/openssl/e_os2.h",
    OPENSSL_DIR + "/include/openssl/err.h", OPENSSL_DIR + "/include/openssl/evperr.h", OPENSSL_DIR + "/include/openssl/evp.h",
    OPENSSL_DIR + "/include/openssl/hmac.h", OPENSSL_DIR + "/include/openssl/idea.h", OPENSSL_DIR + "/include/openssl/kdferr.h",
    OPENSSL_DIR + "/include/openssl/kdf.h", OPENSSL_DIR + "/include/openssl/lhash.h", OPENSSL_DIR + "/include/openssl/md2.h",
    OPENSSL_DIR + "/include/openssl/md4.h", OPENSSL_DIR + "/include/openssl/md5.h", OPENSSL_DIR + "/include/openssl/mdc2.h",
    OPENSSL_DIR + "/include/openssl/modes.h", OPENSSL_DIR + "/include/openssl/objectserr.h", OPENSSL_DIR + "/include/openssl/objects.h",
    OPENSSL_DIR + "/include/openssl/obj_mac.h", OPENSSL_DIR + "/include/openssl/ocsperr.h", OPENSSL_DIR + "/include/openssl/ocsp.h",
    OPENSSL_DIR + "/include/openssl/opensslconf.h", OPENSSL_DIR + "/include/openssl/opensslv.h", OPENSSL_DIR + "/include/openssl/ossl_typ.h",
    OPENSSL_DIR + "/include/openssl/pem2.h", OPENSSL_DIR + "/include/openssl/pemerr.h", OPENSSL_DIR + "/include/openssl/pem.h",
    OPENSSL_DIR + "/include/openssl/pkcs12err.h", OPENSSL_DIR + "/include/openssl/pkcs12.h", OPENSSL_DIR + "/include/openssl/pkcs7err.h",
    OPENSSL_DIR + "/include/openssl/pkcs7.h", OPENSSL_DIR + "/include/openssl/rand_drbg.h", OPENSSL_DIR + "/include/openssl/randerr.h",
    OPENSSL_DIR + "/include/openssl/rand.h", OPENSSL_DIR + "/include/openssl/rc2.h", OPENSSL_DIR + "/include/openssl/rc4.h",
    OPENSSL_DIR + "/include/openssl/rc5.h", OPENSSL_DIR + "/include/openssl/ripemd.h", OPENSSL_DIR + "/include/openssl/rsaerr.h",
    OPENSSL_DIR + "/include/openssl/rsa.h", OPENSSL_DIR + "/include/openssl/safestack.h", OPENSSL_DIR + "/include/openssl/seed.h",
    OPENSSL_DIR + "/include/openssl/sha.h", OPENSSL_DIR + "/include/openssl/srp.h", OPENSSL_DIR + "/include/openssl/srtp.h",
    OPENSSL_DIR + "/include/openssl/ssl2.h", OPENSSL_DIR + "/include/openssl/ssl3.h", OPENSSL_DIR + "/include/openssl/sslerr.h",
    OPENSSL_DIR + "/include/openssl/ssl.h", OPENSSL_DIR + "/include/openssl/stack.h", OPENSSL_DIR + "/include/openssl/storeerr.h",
    OPENSSL_DIR + "/include/openssl/store.h", OPENSSL_DIR + "/include/openssl/symhacks.h", OPENSSL_DIR + "/include/openssl/tls1.h",
    OPENSSL_DIR + "/include/openssl/tserr.h", OPENSSL_DIR + "/include/openssl/ts.h", OPENSSL_DIR + "/include/openssl/txt_db.h",
    OPENSSL_DIR + "/include/openssl/uierr.h", OPENSSL_DIR + "/include/openssl/ui.h", OPENSSL_DIR + "/include/openssl/whrlpool.h",
    OPENSSL_DIR + "/include/openssl/x509err.h", OPENSSL_DIR + "/include/openssl/x509.h", OPENSSL_DIR + "/include/openssl/x509v3err.h",
    OPENSSL_DIR + "/include/openssl/x509v3.h", OPENSSL_DIR + "/include/openssl/x509_vfy.h"
]





##### HELPER FUNCTIONS #####
def supports_color():
    """
        Returns True if the running system's terminal supports color, and False
        otherwise.

        From: https://stackoverflow.com/a/22254892
    """
    plat = sys.platform
    supported_platform = plat != 'Pocket PC' and (plat != 'win32' or
                                                  'ANSICON' in os.environ)
    # isatty is not always implemented, #6223.
    is_a_tty = hasattr(sys.stdout, 'isatty') and sys.stdout.isatty()
    return supported_platform and is_a_tty

def to_bytes(val: int) -> str:
    """
        Pretty-prints the given value to some byte count.
    """

    if val < 1000:
        return f"{val:.2f} bytes"
    elif val < 1000000:
        return f"{val / 1000:.2f} KB"
    elif val < 1000000000:
        return f"{val / 1000000:.2f} MB"
    elif val < 1000000000000:
        return f"{val / 1000000000:.2f} GB"
    elif val < 1000000000000000:
        return f"{val / 1000000000000:.2f} TB"
    else:
        return f"{val / 1000000000000000:.2f} PB"

def perror(text: str = "", colour: bool = True):
    """
        Writes text to stderr, as an Error.
    """

    # Get colours
    start = "\033[91;1m" if colour and supports_color() else ""
    end   = "\033[0m" if colour and supports_color() else ""

    # Print it
    print(f"{start}[ERROR] {text}{end}", file=sys.stderr)

def pwarning(text: str = "", colour: bool = True):
    """
        Writes text to srderr, as a warning string.
    """

    # Get colours
    start = "\033[93;1m" if colour and supports_color() else ""
    end   = "\033[0m" if colour and supports_color() else ""

    # Print it
    print(f"{start}[warning] {text}{end}", file=sys.stderr)

def pdebug(text: str = "", colour: bool = True):
    """
        Writes text to stdout, as a debug string.
    """

    # Skip if not debugging
    if not debug: return

    # Get colours
    start = "\033[90m" if colour and supports_color() else ""
    end   = "\033[0m" if colour and supports_color() else ""

    # Print it
    print(f"{start}[debug] {text}{end}")

def cancel(text: str = "", code = 1, colour: bool = True) -> typing.NoReturn:
    """
        Prints some error message to stderr, then quits the program by calling exit().
    """

    perror(text, colour=colour)
    exit(code)

def resolve_args(text: str, release: str, os_id: str, arch_id: str) -> str:
    """
        Returns the same string, but with a couple of values replaced:
        - `$RELEASE` with 'release' or 'debug' (depending on the '--dev' flag)
        - `$OS` with 'linux' or 'darwin' (based on the '--os' flag)
        - `$ARCH` with 'x86_64' or 'aarch64' (based on the '--arch' flag)
        - `$CWD` with the current working directory (based on what `os.getcwd()` reports)
    """

    return text \
        .replace("$RELEASE", release) \
        .replace("$OS", os_id) \
        .replace("$ARCH", arch_id) \
        .replace("$CWD", os.getcwd())

def needs_recompile(hash_cache: str, src: str) -> bool:
    """
        Checks if the given source file/directory exists and needs
        recompilation.

        It needs recompilation if:
        - It's a directory:
          - Any of its source files (recursively) needs to be recompiled
        - It's a file:
          - The file's hash wasn't cached yet
          - The hashes of the file & directory do not match
        
        Additionally, the user will be warned if the source doesn't exist.
    """

    # Get absolute version of the hash_cache
    hash_cache = os.path.abspath(hash_cache)

    # Match the type of the source file
    if os.path.isfile(src):
        # It's a file; check if we know its hash
        hsrc = os.path.abspath(hash_cache + '/' + src)
        if hsrc[:len(hash_cache)] != hash_cache: cancel(f"Hash source '{hsrc}' is not in the hash cache; please do not escape it")
        if not os.path.exists(hsrc): return True

        # Compute the hash of the file
        try:
            with open(src, "rb") as h:
                src_hash = hashlib.sha256()
                while True:
                    data = h.read(65536)
                    if not data: break
                    src_hash.update(h.read())
        except IOError as e:
            cancel(f"Failed to read source file '{src}'")

        # Compare it with that in the file
        try:
            with open(hsrc, "r") as h:
                cache_hash = h.read()
        except IOError as e:
            cancel(f"Failed to read hash cache file '{hsrc}'")
        if src_hash.hexdigest() != cache_hash: return True

        # Otherwise, no recompilation needed
        return False

    elif os.path.isdir(src):
        # It's a dir; recurse
        for file in os.listdir(src):
            if needs_recompile(hash_cache, os.path.join(src, file)): return True
        return False

    else:
        # Warn the user
        pwarning(f"Source '{src}' not found (is the source list up-to-date?)")
        return False

def update_cache(hash_cache: str, src: str):
    """
        Updates the hash of the given source file in the given hash cache.
        If the src is a file, then we simply compute the hash.
        We recurse if it's a directory.
    """

    # Get absolute version of the hash_cache
    hash_cache = os.path.abspath(hash_cache)

    # Match the type of the source file
    if os.path.isfile(src):
        # Attempt to compute the hash
        try:
            with open(src, "rb") as h:
                src_hash = hashlib.sha256()
                while True:
                    data = h.read(65536)
                    if not data: break
                    src_hash.update(h.read())
        except IOError as e:
            cancel(f"Failed to read source file '{src}'")

        # Check if the target directory exists
        hsrc = os.path.abspath(hash_cache + '/' + src)
        if hsrc[:len(hash_cache)] != hash_cache: cancel(f"Hash source '{hsrc}' is not in the hash cache; please do not escape it")
        if not os.path.exists(os.path.dirname(hsrc)):
            os.makedirs(os.path.dirname(hsrc))

        # Write the hash to it
        try:
            with open(hsrc, "w") as h:
                h.write(src_hash.hexdigest())
        except IOError as e:
            cancel(f"Failed to write hash cache to '{hsrc}'")

    elif os.path.isdir(src):
        # It's a dir; recurse
        for file in os.listdir(src):
            if update_cache(hash_cache, os.path.join(src, file)): return True
        return False

    else:
        # Warn the user
        pwarning(f"Source '{src}' not found (is the source list up-to-date?)")
        return False

def deduce_toml_src_dirs(toml: str) -> list[str] | None:
    """
        Given a Cargo.toml file, attempts to deduce the (local) source crates.

        Returns a list of the folders that are the crates on which the
        Cargo.toml depends, including the one where it lives (i.e., its
        directory-part).
    """

    res = [ os.path.dirname(toml) ]

    # Scan the lines in the file
    try:
        with open(toml, "r") as h:
            # Read it all
            text = h.read()

            # Parse
            parser = CargoTomlParser(text)
            (res, errs) = parser.parse()
            if len(errs) > 0:
                for err in errs:
                    perror(f"{err}")
                return None
            
            # Else, resolve the given paths
            for i in range(len(res)):
                res[i] = os.path.join(os.path.dirname(toml), res[i])
            # Add the cargo path
            res.append(os.path.dirname(toml))
            # Make all paths absolute
            for i in range(len(res)):
                res[i] = os.path.abspath(res[i])

            # Done
            return res

    except IOError as e:
        cancel(f"Could not read given Cargo.toml '{toml}': {e}")

def get_image_digest(path: str) -> str:
    """
        Given a Docker image .tar file, attempts to read the digest and return it.
    """

    # Open the tar file
    archive = tarfile.open(path)

    # Find the manifest file
    digest = None
    for file in archive.getmembers():
        # Skip if not the proper file
        if not file.isfile() or file.name != "manifest.json": continue

        # Attempt to read it
        f = archive.extractfile(file)
        manifest = f.read().decode("utf-8")
        f.close()
        
        # Read as json
        manifest = json.loads(manifest)

        # Extract the config blob (minus prefix)
        config = manifest[0]["Config"]
        if config[:13] != "blobs/sha256/": cancel("Found Config in manifest.json, but blob had incorrect start (corrupted image .tar?)")
        config = config[13:]

        # Done
        digest = config

    # Throw a failure
    if digest is None:
        cancel(f"Did not find image digest in {path} (is it a valid Docker image file?)")

    # Done
    archive.close()
    return digest

def build_dep(args: argparse.Namespace, dep_name: str, building: set[str]):
    """
        Builds the given dependencies and its dependencies.
    """

    # Get the actual target behind the name
    if dep_name not in targets:
        cancel(f"Unknown target '{dep_name}'", file=sys.stderr)
    dep = targets[dep_name]

    # Only build if not already commited to building or no recompilation necessary
    if dep_name in building or not dep.check_regen(args): return

    # Build it
    dep.build(args, _building=building)



##### HELPER CLASSES #####
class ForceOrPrecompiled(argparse.Action):
    """
        Defines a custom argparse action that allows us to retain the last
        value of either '--force' or '--precompiled'.

        Based on: https://stackoverflow.com/a/9028031
    """


    def __call__(self, parser, namespace, values, option_string=None):
        # If the destination is either of our special flags, hit it
        if self.dest == "force":
            # Set the flags appropriately
            setattr(namespace, "force", True)
            setattr(namespace, "no_compile", False)
        elif self.dest == "no_compile":
            # Set the flags appropriately
            setattr(namespace, "force", False)
            setattr(namespace, "no_compile", True)
        else:
            raise ValueError(f"Please use {self.__name__} only for the '--force' and '--no-compile' flags")

class ProgressBar:
    """
        Class that shows a simply progress bar on the CLI.
    """

    _width     : int
    _i         : int
    _max       : int
    _prefix    : str
    _draw_time : float
    _last_draw : float


    def __init__(self, start: int=0, stop: int=99, prefix: str="", width: int=None, draw_time: float=0.5) -> None:
        """
            Constructor for the ProgressBar class.

            Arguments:
            - `start`: The start value of the progressbar (before calling update() or set()).
            - `stop`: The end value. As soon as update() or set() pushes the value equal to (or above) this one, the progress bar will reach 100%.
            - `prefix`: Some extra text to preview at the start of the bar.
            - `width`: The width (in characters) or the progress bar. If 'None', tries to deduce it automatically (using ).
            - `draw_time`: The time (in seconds) between two draw calls.
        """

        # Deduce the wdith
        if hasattr(sys.stdout, 'isatty') and sys.stdout.isatty():
            width = width if width is not None else os.get_terminal_size().columns

        # Set the values
        self._width     = width
        self._i         = start
        self._max       = stop
        self._prefix    = prefix
        self._last_bin  = -10
        self._draw_time = draw_time
        self._last_draw = 0



    def draw(self) -> None:
        """
            Re-draws the progress bar by going to the start of the line (using '\r') and drawing it.

            Any potential 'draw timing' (i.e., only updating the terminal every half a second or so) should be done when calling this function.
        """

        # Switch on whether the terminal is a tty
        if hasattr(sys.stdout, 'isatty') and sys.stdout.isatty():
            # Compute the non-prefix width
            width = self._width - len(self._prefix)

            # Write the prefix first
            print(f"\r{self._prefix}", end="")

            # Write the start of the bar
            if width < 1: return
            print(f"[", end="")

            # Now write the bar itself to a string of the appropriate height
            if width > 2:
                bar_end = int((self._i / self._max) * (width - 2)) if self._i < self._max else (width - 2)
                bar = "=" * bar_end + " " * ((width - 2) - bar_end)

                # Overwrite the middle part with the progress percentage
                percentage = self._i / self._max * 100
                percentage = f"{percentage:.1f}%"
                if len(bar) >= len(percentage):
                    percentage_start = (len(bar) // 2) - (len(percentage) // 2)
                    bar = bar[:percentage_start] + percentage + bar[percentage_start + len(percentage):]
                print(bar, end="")

            # Write the end thingy
            if width < 2: return
            print(f"]", end="")
        else:
            # Simply write the progress bin we just reached
            percentage = self._i / self._max * 100
            if percentage - self._last_bin >= 10:
                self._last_bin += 10
                print(f"{self._prefix if self._last_bin < 0 else ''}{self._last_bin}{'...' if self._last_bin < 100 else ''}", end="")

        # Don't forget to flush stdout
        sys.stdout.flush()

    def update(self, step=1, force_draw=False) -> None:
        """
            Updates the progress bar with the given number of steps (i.e., relative update).

            If `force_draw` is False, only redraws every half second. Otherwise, always draws when update() is called.
        """

        # Update the value
        self._i += step
        if self._i < 0: self._i = 0
        if self._i > self._max: self._i = self._max

        # Redraw if necessary
        if force_draw or time.time() - self._last_draw > self._draw_time:
            self.draw()
            self._last_draw = time.time()

    def update_to(self, i, force_draw=False) -> None:
        """
            Sets the progress bar to the given amount of value (i.e., absolute update).

            If `force_draw` is False, only redraws every half second. Otherwise, always draws when update() is called.
        """

        # Update the value
        self._i = i
        if self._i < 0: self._i = 0
        if self._i > self._max: self._i = self._max

        # Redraw if necessary
        if force_draw or time.time() - self._last_draw > self._draw_time:
            self.draw()
            self._last_draw = time.time()

    def stop(self) -> None:
        """
            Stops the progress bar by writing a newline.

            Always draws, then stops drawing forever.
        """

        self._i = self._max
        self.draw()
        self._last_draw = sys.maxsize * 2 + 1
        print()



    def update_prefix(self, prefix) -> None:
        """
            Changes the prefix before the progress bar.
        """

        self._prefix = prefix

class CargoTomlParser:
    """
        Parses a given file as if it were a Cargo.toml file.

        This is definitely not a TOML compliant-parser, though, not least of
        which because it only extracts stuff under the 'dependencies' toplevel
        section.
    """


    # Baseclasses
    class Symbol(abc.ABC):
        """
            Baseclass for all the symbols.
        """

        is_term : bool
        start   : tuple[int, int]
        end     : tuple[int, int]


        def __init__(self, is_term: bool, start: tuple[int, int], end: tuple[int, int]) -> None:
            """
                Constructor for the Symbol.

                # Arguments
                - `is_term`: Whether this Symbol is a terminal or not (it's a nonterminal).
                - `start`: The (inclusive) start position of this symbol in the text.
                - `stop`: The (inclusive) stop position of this symbol in the text.
            """

            self.is_term = is_term
            self.start   = start
            self.end     = end

        def __str__(self) -> str:
            return "Symbol"

    class Terminal(Symbol):
        """
            Baseclass for all the parser tokens.
        """

        def __init__(self, start: tuple[int, int], end: tuple[int, int]):
            """
                Constructor for the Terminal.

                # Arguments
                - `start`: The (inclusive) start position of this symbol in the text.
                - `end`: The (inclusive) stop position of this symbol in the text.
            """

            CargoTomlParser.Symbol.__init__(self, True, start, end)

        def __str__(self) -> str:
            return "Terminal"

    class Nonterminal(Symbol):
        """
            Baseclass for all the parser nonterminals.
        """

        def __init__(self, start: tuple[int, int], end: tuple[int, int]):
            """
                Constructor for the Nonterminal.

                # Arguments
                - `start`: The (inclusive) start position of this symbol in the text.
                - `end`: The (inclusive) stop position of this symbol in the text.
            """

            CargoTomlParser.Symbol.__init__(self, False, start, end)

        def __str__(self) -> str:
            return "NonTerminal"


    # Terminals
    class Identifier(Terminal):
        """
            Helper class for the CargoTomlParser which represents a string token.
        """

        value : str


        def __init__(self, value: str, start: tuple[int, int], end: tuple[int, int]) -> None:
            """
                Constructor for the String

                Arguments
                - `value`: The value of the String.
                - `start`: The start position (as (row, col), inclusive) of the token.
                - `end`: The end position (as (row, col), inclusive) of the token.
            """

            CargoTomlParser.Terminal.__init__(self, start, end)

            self.value = value

        def __str__(self) -> str:
            return f"Identifier({self.value})"

    class String(Terminal):
        """
            Helper class for the CargoTomlParser which represents a string value.
        """

        value : str


        def __init__(self, value: str, start: tuple[int, int], end: tuple[int, int]) -> None:
            """
                Constructor for the String

                Arguments
                - `value`: The value of the String.
                - `start`: The start position (as (row, col), inclusive) of the token.
                - `end`: The end position (as (row, col), inclusive) of the token.
            """

            CargoTomlParser.Terminal.__init__(self, start, end)

            self.value = value

        def __str__(self) -> str:
            return f"String({self.value})"

    class Boolean(Terminal):
        """
            Helper class for the CargoTomlParser which represents a boolean value.
        """

        value : bool


        def __init__(self, value: bool, start: tuple[int, int], end: tuple[int, int]) -> None:
            """
                Constructor for the Boolean

                Arguments
                - `value`: The value of the Boolean.
                - `start`: The start position (as (row, col), inclusive) of the token.
                - `end`: The end position (as (row, col), inclusive) of the token.
            """

            CargoTomlParser.Terminal.__init__(self, start, end)

            self.value = value

        def __str__(self) -> str:
            return f"Boolean({self.value})"

    class Equals(Terminal):
        """
            Helper class for the CargoTomlParser which represents an equals sign.
        """


        def __init__(self, start: tuple[int, int], end: tuple[int, int]) -> None:
            """
                Constructor for the Equals

                Arguments
                - `start`: The start position (as (row, col), inclusive) of the token.
                - `end`: The end position (as (row, col), inclusive) of the token.
            """

            CargoTomlParser.Terminal.__init__(self, start, end)

        def __str__(self) -> str:
            return "Equals"

    class Comma(Terminal):
        """
            Helper class for the CargoTomlParser which represents a comma.
        """


        def __init__(self, start: tuple[int, int], end: tuple[int, int]) -> None:
            """
                Constructor for the Comma

                Arguments
                - `start`: The start position (as (row, col), inclusive) of the token.
                - `end`: The end position (as (row, col), inclusive) of the token.
            """

            CargoTomlParser.Terminal.__init__(self, start, end)

        def __str__(self) -> str:
            return "Comma"

    class LCurly(Terminal):
        """
            Helper class for the CargoTomlParser which represents a left curly bracket.
        """


        def __init__(self, start: tuple[int, int], end: tuple[int, int]) -> None:
            """
                Constructor for the LCurly

                Arguments
                - `start`: The start position (as (row, col), inclusive) of the token.
                - `end`: The end position (as (row, col), inclusive) of the token.
            """

            CargoTomlParser.Terminal.__init__(self, start, end)

        def __str__(self) -> str:
            return "LCurly"

    class RCurly(Terminal):
        """
            Helper class for the CargoTomlParser which represents a right curly bracket.
        """


        def __init__(self, start: tuple[int, int], end: tuple[int, int]) -> None:
            """
                Constructor for the RCurly

                Arguments
                - `start`: The start position (as (row, col), inclusive) of the token.
                - `end`: The end position (as (row, col), inclusive) of the token.
            """

            CargoTomlParser.Terminal.__init__(self, start, end)

        def __str__(self) -> str:
            return "RCurly"

    class LSquare(Terminal):
        """
            Helper class for the CargoTomlParser which represents a left square bracket.
        """


        def __init__(self, start: tuple[int, int], end: tuple[int, int]) -> None:
            """
                Constructor for the LSquare

                Arguments
                - `start`: The start position (as (row, col), inclusive) of the token.
                - `end`: The end position (as (row, col), inclusive) of the token.
            """

            CargoTomlParser.Terminal.__init__(self, start, end)

        def __str__(self) -> str:
            return "LSquare"

    class RSquare(Terminal):
        """
            Helper class for the CargoTomlParser which represents a right square bracket.
        """


        def __init__(self, start: tuple[int, int], end: tuple[int, int]) -> None:
            """
                Constructor for the RSquare

                Arguments
                - `start`: The start position (as (row, col), inclusive) of the token.
                - `end`: The end position (as (row, col), inclusive) of the token.
            """

            CargoTomlParser.Terminal.__init__(self, start, end)

        def __str__(self) -> str:
            return "RSquare"


    # Nonterminals
    class Section(Nonterminal):
        """
            Represents a section in the TOML file.
        """

        header : CargoTomlParser.SectionHeader
        pairs  : list[CargoTomlParser.KeyValuePair]


        def __init__(self, header: CargoTomlParser.SectionHeader, pairs: list[CargoTomlParser.KeyValuePair], start: tuple[int, int], end: tuple[int, int]):
            """
                Constructor for the SectionHeader nonterminal.

                # Arguments
                - `header`: The header of the section.
                - `pairs`: The key/value pairs in this section.
                - `start`: The (inclusive) start position of this token.
                - `end`: The (inclusive) end position of this token.
            """

            CargoTomlParser.Nonterminal.__init__(self, start, end)

            self.header = header
            self.pairs  = pairs

        def __str__(self) -> str:
            return f"Section({self.header}, ...)"

    class SectionHeader(Nonterminal):
        """
            Represents a section header.
        """

        name : str

        def __init__(self, name: str, start: tuple[int, int], end: tuple[int, int]):
            """
                Constructor for the SectionHeader nonterminal.

                # Arguments
                - `name`: The name of the section.
                - `start`: The (inclusive) start position of this token.
                - `end`: The (inclusive) end position of this token.
            """

            CargoTomlParser.Nonterminal.__init__(self, start, end)

            self.name = name

        def __str__(self) -> str:
            return f"SectionHeader({self.name})"

    class KeyValuePair(Nonterminal):
        """
            Represents a Key/Value pair in the stack.
        """

        key   : CargoTomlParser.Identifier
        value : CargoTomlParser.Value


        def __init__(self, key: CargoTomlParser.Identifier, value: CargoTomlParser.Value, start: tuple[int, int], end: tuple[int, int]):
            """
                Constructor for the SectionHeader nonterminal.

                # Arguments
                - `key`: The key of the pair (which is an Identifier).
                - `value`: The value of the pair (which is a Value).
                - `start`: The (inclusive) start position of this token.
                - `end`: The (inclusive) end position of this token.
            """

            CargoTomlParser.Nonterminal.__init__(self, start, end)

            self.key   = key
            self.value = value

        def __str__(self) -> str:
            return f"KeyValuePair({self.key}, {self.value})"

    class Value(Nonterminal):
        """
            Abstracts away over the specific value.
        """

        value : CargoTomlParser.String

        def __init__(self, value: CargoTomlParser.String, start: tuple[int, int], end: tuple[int, int]):
            """
                Constructor for the SectionHeader nonterminal.

                # Arguments
                - `value`: The value of the Value.
                - `start`: The (inclusive) start position of this token.
                - `end`: The (inclusive) end position of this token.
            """

            CargoTomlParser.Nonterminal.__init__(self, start, end)

            self.value = value

        def __str__(self) -> str:
            return f"Value({self.value})"

    class Dict(Nonterminal):
        """
            Represents a dictionary of key/value pairs.
        """

        pairs : list[CargoTomlParser.KeyValuePair]


        def __init__(self, pairs: list[CargoTomlParser.KeyValuePair], start: tuple[int, int], end: tuple[int, int]):
            """
                Constructor for the SectionHeader nonterminal.

                # Arguments
                - `pairs`: The list of KeyValuePairs in this dictionary.
                - `start`: The (inclusive) start position of this token.
                - `end`: The (inclusive) end position of this token.
            """

            CargoTomlParser.Nonterminal.__init__(self, start, end)

            self.pairs = pairs

        def __str__(self) -> str:
            res = "Dict({\n"
            for p in self.pairs:
                res += f"    {p},\n"
            return res + "})"

    class List(Nonterminal):
        """
            Represents a list of values.
        """

        values : list[Value]


        def __init__(self, values: list[Value], start: tuple[int, int], end: tuple[int, int]):
            """
                Constructor for the SectionHeader nonterminal.

                # Arguments
                - `values`: The list of Values in this list.
                - `start`: The (inclusive) start position of this token.
                - `end`: The (inclusive) end position of this token.
            """

            CargoTomlParser.Nonterminal.__init__(self, start, end)

            self.values = values

        def __str__(self) -> str:
            res = "List(["
            for i, v in enumerate(self.values):
                if i > 0: res += ", "
                res += f"{v}"
            return res + "])"



    _lines : str
    _col   : int
    _line  : int


    def __init__(self, raw: str) -> None:
        """
            Constructor for the CargoTomlParser.

            Arguments:
            - `raw`: The raw text to parse as a Cargo.toml file.
        """

        # Initialize stuff
        self._lines = raw.split('\n')
        self._col   = 0
        self._line  = 0

    def _token(self) -> Terminal | Exception | None:
        """
            Consumes the next token from the internal text.

            If the returned value derived from an Exception, then the text is
            invalid TOML.
            If the returned value is None, then no more tokens are available.
        """

        start = (0, 0)
        buffer = ""
        mode = "start"
        while self._line < len(self._lines):
            if self._col >= len(self._lines[self._line]):
                # Update the values
                self._col = 0
                self._line += 1

                # Throw errors where it matters
                if mode == "identifier":
                    return CargoTomlParser.Identifier(buffer, start, (self._line - 1, len(self._lines[self._line - 1]) - 1))
                elif mode == "section":
                    return ValueError(f"{self._line}:{self._col}: Encountered unterminated section header (missing ']')")
                elif mode == "string":
                    return ValueError(f"{self._line}:{self._col}: Encountered unterminated string (missing '\"')")
                elif mode == "string_escape":
                    return ValueError(f"{self._line}:{self._col}: Missing escape character")
                elif mode == "comment":
                    # Go back to start mode
                    mode = "start"
            if self._line >= len(self._lines):
                break
            if self._col  >= len(self._lines[self._line]):
                continue
            c = self._lines[self._line][self._col]
            # print(f"\n >>> [{mode}] CHAR {self._line}:{self._col}: '{c}'")

            # Switch on the mode
            if mode == "start":
                start = (self._line, self._col)

                # Switch on the character
                if (ord(c) >= ord('a') and ord(c) <= ord('z')) or (ord(c) >= ord('A') and ord(c) <= ord('Z')) or c == '_':
                    # Switch to parsing an identifier token
                    mode = "identifier"
                    buffer += c
                    self._col += 1
                    continue
                elif c == '\'' or c == '"':
                    # Switch to parsing as string literal
                    mode = "string"
                    self._col += 1
                    continue
                elif c == '=':
                    # Just parse as an equals-sign
                    self._col += 1
                    return CargoTomlParser.Equals(start, start)
                elif c == ',':
                    # Just parse as a comma
                    self._col += 1
                    return CargoTomlParser.Comma(start, start)
                elif c == '{':
                    # Return the character as a token
                    self._col += 1
                    return CargoTomlParser.LCurly(start, start)
                elif c == '}':
                    # Return the character as a token
                    self._col += 1
                    return CargoTomlParser.RCurly(start, start)
                elif c == '[':
                    # Simply return the LBracket
                    self._col += 1
                    return CargoTomlParser.LSquare(start, start)
                elif c == ']':
                    # Simply return the RBracket
                    self._col += 1
                    return CargoTomlParser.RSquare(start, start)
                elif c == ' ' or c == '\t' or c == '\r':
                    # Skip
                    self._col += 1
                    continue
                elif c == '#':
                    # Comment
                    mode = "comment"
                    self._col += 1
                    continue
                else:
                    self._col += 1
                    return ValueError(f"{start[0]}:{start[1]}: Unexpected character '{c}'")

            elif mode == "identifier":
                # Switch on the character
                if (ord(c) >= ord('a') and ord(c) <= ord('z')) or \
                   (ord(c) >= ord('A') and ord(c) <= ord('Z')) or \
                   (ord(c) >= ord('0') and ord(c) <= ord('9')) or \
                    c == '-' or c == '_':
                    # Keep parsing
                    buffer += c
                    self._col += 1
                    continue
                else:
                    # Done parsing; let start handle this char

                    # If keyword, intercept that
                    if buffer == "true" or buffer == "false":
                        # It's a boolean instead
                        return CargoTomlParser.Boolean(buffer == "true", start, (self._line, self._col - 1))

                    # Otherwise, identifier
                    return CargoTomlParser.Identifier(buffer, start, (self._line, self._col - 1))

            elif mode == "string":
                # Switch on the character
                if c == '\\':
                    # Parse as escaped string
                    mode = "string_escape"
                    self._col += 1
                    continue
                elif c == '"':
                    # We're done!
                    self._col += 1
                    return CargoTomlParser.String(buffer, start, (self._line, self._col - 1))
                else:
                    # Parse as part of the token
                    buffer += c
                    self._col += 1
                    continue

            elif mode == "string_escape":
                # Switch on the character
                if c == '\\' or c == '"' or c == '\'':
                    buffer += c
                    mode = "string"
                    self._col += 1
                    continue
                elif c == 'n':
                    buffer += '\n'
                    mode = "string"
                    self._col += 1
                    continue
                elif c == 't':
                    buffer += '\t'
                    mode = "string"
                    self._col += 1
                    continue
                elif c == 'r':
                    buffer += '\r'
                    mode = "string"
                    self._col += 1
                    continue
                else:
                    # Ignore
                    perror(f"{self._line}:{self._col}: Unknown escape character '{c}' (ignoring)")
                    mode = "string"
                    self._col += 1
                    continue

            elif mode == "comment":
                # Do nothing
                self._col += 1
                continue

            else:
                raise ValueError(f"Unknown mode '{mode}'; this should never happen!")
        return None

    def _reduce(self, stack: list[Symbol]) -> tuple[list[Symbol], str | None | Exception]:
        """
            Attempts to apply one of the reduction rules to the current stack of tokens.

            Upon success, returns some string that identifies the rule applied.
            If no rule has been applied, returns None.
            Upon failure, returns an Exception.
        """

        # Start at the end, go backwards to identify the rules
        mode = "start"
        i = len(stack) - 1
        list_values = []
        while i >= 0:
            # Get the current symbol
            s = stack[i]

            # Match the mode
            if mode == "start":
                # Switch between token or not
                if s.is_term:
                    # Match the type of it
                    if type(s) == CargoTomlParser.RSquare:
                        # Might be a section header or a list!
                        mode = "rsquare"
                        i -= 1
                        continue

                    elif type(s) == CargoTomlParser.RCurly:
                        # Might be a dict!
                        mode = "dict"
                        i -= 1
                        continue

                    elif type(s) == CargoTomlParser.String:
                        # Immediately cast to a value
                        return (stack[:i] + [ CargoTomlParser.Value(s, s.start, s.end) ], "value_string")

                    elif type(s) == CargoTomlParser.Boolean:
                        # Immediately cast to a value
                        return (stack[:i] + [ CargoTomlParser.Value(s, s.start, s.end) ], "value_boolean")

                    else:
                        # No rule (yet)
                        return (stack, None)

                else:
                    # Match the type of it
                    if type(s) == CargoTomlParser.SectionHeader:
                        # Cast to a Section
                        return (stack[:i] + [ CargoTomlParser.Section(s, [], s.start, s.end) ], "section_header")

                    elif type(s) == CargoTomlParser.KeyValuePair:
                        # See if it is preceded by a Section
                        mode = "key_value_pair"
                        i -= 1
                        continue

                    elif type(s) == CargoTomlParser.Value:
                        # Might be a key/value pair
                        mode = "value"
                        i -= 1
                        continue

                    elif type(s) == CargoTomlParser.List:
                        # Cast to a value
                        return (stack[:i] + [ CargoTomlParser.Value(s, s.start, s.end) ], "value_list")

                    elif type(s) == CargoTomlParser.Dict:
                        # Cast to a value
                        return (stack[:i] + [ CargoTomlParser.Value(s, s.start, s.end) ], "value_dict")

                    else:
                        # No rule (yet)
                        return (stack, None)

            elif mode == "key_value_pair":
                # Switch between token or not
                if s.is_term:
                    # Ignore
                    return (stack, None)

                else:
                    # Match the type of it
                    if type(s) == CargoTomlParser.Section:
                        # Append to the section
                        s.pairs.append(stack[i + 1])
                        s.end = stack[i + 1].end
                        return (stack[:i + 1], "section_append")

                    else:
                        # No rule (yet)
                        return (stack, None)

            elif mode == "rsquare":
                # Switch between token or not
                if s.is_term:
                    # Match the type of it
                    if type(s) == CargoTomlParser.Identifier:
                        # Yes, on the road to section header!
                        mode = "rsquare_ident"
                        i -= 1
                        continue

                    elif type(s) == CargoTomlParser.LSquare:
                        # Empty list, we can only assume
                        new_s = CargoTomlParser.List([], stack[i].start, stack[i + 1].end)
                        return (stack[:i] + [ new_s ], "empty-list")

                    else:
                        # No rule (yet)
                        return (stack, None)

                else:
                    # Match the type of it
                    if type(s) == CargoTomlParser.Value:
                        # It must be the start of a list
                        mode = "list"
                        continue

                    else:
                        # No rule (yet)
                        return (stack, None)

            elif mode == "rsquare_ident":
                # Switch between token or not
                if s.is_term:
                    # Match the type of it
                    if type(s) == CargoTomlParser.LSquare:
                        # Whohoo, replace them in the stack (reduce)
                        new_s = CargoTomlParser.SectionHeader(stack[i + 1].value, stack[i + 2].start, stack[i].end)
                        return (stack[:i] + [ new_s ], "section-header")

                    else:
                        # No rule (yet)
                        return (stack, None)

                else:
                    # No rule (yet)
                    return (stack, None)

            elif mode == "dict":
                # Switch between token or not
                if s.is_term:
                    # Match the type of it
                    if type(s) == CargoTomlParser.LCurly:
                        # It's an empty dict
                        new_s = CargoTomlParser.Dict([], stack[i].start, stack[i + 1].end)
                        return (stack[:i] + [ new_s ], "empty-dict")

                    else:
                        return (stack[:i], ValueError(f"Invalid dict entry: expected a key/value pair, got {s}"))

                else:
                    # Match the type of it
                    if type(s) == CargoTomlParser.KeyValuePair:
                        # It's a key/value pair; start parsing it as such
                        list_values.append(s)
                        mode = "dict_pair"
                        i -= 1
                        continue

                    else:
                        return (stack[:i], ValueError(f"Invalid dict entry: expected a key/value pair, got {s}"))

            elif mode == "dict_pair":
                # Switch between token or not
                if s.is_term:
                    # Match the type of it
                    if type(s) == CargoTomlParser.LCurly:
                        # End of the list
                        list_values.reverse()
                        new_s = CargoTomlParser.Dict(list_values, stack[i].start, stack[len(stack) - 1].end)
                        return (stack[:i] + [ new_s ], "dict")

                    elif type(s) == CargoTomlParser.Comma:
                        # The list continious
                        mode = "dict"
                        i -= 1
                        continue

                    else:
                        return (stack[:i], ValueError(f"Invalid dict: expected ',' or '{{', got {s}"))

                else:
                    # We don't accept any nonterminals at this stage
                    return (stack[:i], ValueError(f"Invalid list: expected ',' or '[', got {s}"))

            elif mode == "list":
                # Switch between token or not
                if s.is_term:
                    # Match the type of it
                    if len(list_values) == 0 and type(s) == CargoTomlParser.LSquare:
                        # End of the list, but it is empty
                        new_s = CargoTomlParser.List([], stack[i].start, stack[i + 1].end)
                        return (stack[:i] + [ new_s ], "empty-list")

                    else:
                        return (stack[:i], ValueError(f"Invalid list entry: expected a Value, got {s}"))

                else:
                    # Match the type of it
                    if type(s) == CargoTomlParser.Value:
                        # Yes, keep parsing
                        list_values.append(s)
                        mode = "list_value"
                        i -= 1
                        continue

                    else:
                        return (stack[:i], ValueError(f"Invalid list entry: expected a Value, got {s}"))

            elif mode == "list_value":
                # Switch between token or not
                if s.is_term:
                    # Match the type of it
                    if type(s) == CargoTomlParser.LSquare:
                        # End of the list
                        list_values.reverse()
                        new_s = CargoTomlParser.List(list_values, stack[i].start, stack[len(stack) - 1].end)
                        return (stack[:i] + [ new_s ], "list")

                    elif type(s) == CargoTomlParser.Comma:
                        # The list continious
                        mode = "list"
                        i -= 1
                        continue

                    else:
                        return (stack[:i], ValueError(f"Invalid list: expected ',' or '[', got {s}"))

                else:
                    # We don't accept any nonterminals at this stage
                    return (stack[:i], ValueError(f"Invalid list: expected ',' or '[', got {s}"))

            elif mode == "value":
                # Switch between token or not
                if s.is_term:
                    # Match the type of it
                    if type(s) == CargoTomlParser.Equals:
                        # Yes, good going!
                        mode = "value_equals"
                        i -= 1
                        continue

                    else:
                        # No rule (yet)
                        return (stack, None)

                else:
                    # No rule (yet)
                    return (stack, None)

            elif mode == "value_equals":
                # Switch between token or not
                if s.is_term:
                    # Match the type of it
                    if type(s) == CargoTomlParser.Identifier:
                        # It's a key/value pair
                        new_s = CargoTomlParser.KeyValuePair(stack[i], stack[i + 2], stack[i].start, stack[i + 2].end)
                        return (stack[:i] + [ new_s ], "key-value-pair")

                    else:
                        # No rule (yet)
                        return (stack, None)

                else:
                    # No rule (yet)
                    return (stack, None)

            else:
                raise ValueError(f"Unknown mode '{mode}'; this should never happen!")

        # Nothing to be done
        return (stack, None)


    def parse(self) -> tuple[list[str], list[Exception]]:
        """
            Parses the internal Cargo.toml file to extract the list of
            dependencies from it.

            Returns a list with the depedency folders of the given Cargo.toml,
            excluding that of the Cargo.toml itself.
        """

        # Parse the tokens using a shift-reduce parser
        errs = []
        stack = []
        while True:
            # Get a new token
            token = self._token()

            # Store errors for printing
            if isinstance(token, Exception):
                errs.append(token)
                continue
            if token is None:
                break

            # Push it on the stack (shift)
            stack.append(token)
            # print("Shifted")

            # # Print the stack (debug)
            # print('[', end="")
            # for (i, s) in enumerate(stack):
            #     if i > 0: print(" ", end="")
            #     print(f"{s}", end="")
            # print(']\n'); 

            # Now, attempt to (reduce) as much as possible
            while True:
                (stack, rule) = self._reduce(stack)
                if isinstance(rule, Exception):
                    errs.append(rule)
                    continue
                if rule is None:
                    break
                # print(f"Applied rule '{rule}'")

                # # Print the stack (debug)
                # print('[', end="")
                # for (i, s) in enumerate(stack):
                #     if i > 0: print(" ", end="")
                #     print(f"{s}", end="")
                # print(']\n');

        # Now, in the parsed struct, attempt to extract the local crates
        paths = []
        for section in stack:
            # Make sure everything was parsed to a section
            if type(section) != CargoTomlParser.Section:
                errs.append(ValueError(f"Encountered stray symbol '{section}'"))
                continue

            # Ignore any non-dependency section
            if section.header.name != "dependencies" and section.header.name != "build-dependencies": continue

            # Iterate over the pairs
            for dependency in section.pairs:
                # Skip it the value is not a dict
                if type(dependency.value.value) != CargoTomlParser.Dict: continue

                # Search the dict for a 'path' identifier
                for pair in dependency.value.value.pairs:
                    if pair.key.value != "path": continue

                    # Store the found path as a dependency folder
                    paths.append(pair.value.value.value)

        # Return the result
        return (paths, errs)





class Arch:
    """
        Defines a wrapper around architecture strings (to handle multiple
        aliases).
    """

    _arch      : str
    _is_given  : bool
    _is_native : bool


    def __init__(self) -> None:
        # Don't reall do anything; just initialize an empty object
        pass

    def new(raw: str) -> Arch:
        """
            Constructs a new Arch that is initialize from the given string.
        """

        # Get an empty object
        arch = Arch()

        # Set the given values (casting them to set strings)
        arch._arch = Arch.resolve(raw)

        # Set the properties a priori
        arch._is_given  = True
        arch._is_native = arch._arch == Arch.host()._arch

        # Done!
        return arch

    def host() -> Arch:
        """
            Returns a new Arch structure that is equal to the one running on the current machine.

            Uses "uname -m" to detect this.
        """

        # Open the process
        handle = subprocess.Popen(["uname", "-m"], stdout=subprocess.PIPE, text=True)
        stdout, _ = handle.communicate()

        # Parse the value, put it in an empty Arch object
        arch = Arch()
        arch._arch = Arch.resolve(stdout)

        # Overrride the propreties, then return
        arch._is_given  = False
        arch._is_native = True
        return arch



    def __str__(self) -> str:
        """
            Returns the 'canonical' / human readable version of the Architecture.
        """

        return self._arch



    def resolve(text: str) -> str:
        """
            Resolves the given architecture string to a valid Arch internal string.
        """

        # Get a more forgiving version of the string
        arch = text.lower().strip()

        # Cast it to the appropriate type or error
        if arch == "x86_64" or arch == "amd64":
            return "x86_64"
        elif arch == "aarch64" or arch == "arm64":
            return "aarch64"
        else:
            raise ValueError(f"Unknown architecture '{text}'")

    def is_given(self) -> bool:
        """
            Returns whether or not the architecture is given manually or simply the host arch.
        """

        return self._is_given

    def is_native(self) -> bool:
        """
            Returns whether or not the current architecture is native.
        """

        return self._is_native



    def to_rust(self) -> str:
        """
            Returns the architecture in a way that is compatible with Rust.
        """

        return self._arch

    def to_docker(self) -> str:
        """
            Returns the architecture in a way that is compatible with Docker.
        """

        return self._arch

    def to_juicefs(self) -> str:
        """
            Returns the architecture in a way that is compatible with the JuiceFS image.
        """

        if self._arch == "x86_64": return "amd64"
        else: return "arm64"

class Os:
    """
        Defines a wrapper around an OS string.
    """

    _os        : str
    _is_given  : bool
    _is_native : bool


    def __init__(self) -> None:
        """
            Initializes an 'empty' Os object.
        """
        pass

    def new(raw: str) -> Os:
        """
            Constructor for the Os object.

            Arguments:
            - `raw`: The raw OS string to parse.
        """

        # Get an empty object
        os = Os()

        # Set the given values (casting them to set strings)
        os._os = Os.resolve(raw)

        # Set the properties a priori
        os._is_given  = True
        os._is_native = os._os == Os.host()._os

        # Done!
        return os

    def host() -> Arch:
        """
            Returns a new Os structure that is equal to the one running on the current machine.

            Uses "uname -s" to detect this.
        """

        # Open the process
        handle = subprocess.Popen(["uname", "-s"], stdout=subprocess.PIPE, text=True)
        stdout, _ = handle.communicate()

        # Parse the value, put it in an empty Arch object
        os = Os()
        os._os = Os.resolve(stdout)

        # Overrride the propreties, then return
        os._is_given  = False
        os._is_native = True
        return os



    def __str__(self) -> str:
        """
            Returns the 'canonical' / human readable version of the Os.
        """

        return self._os



    def resolve(text: str) -> str:
        """
            Resolves the given OS string to a valid Os internal string.
        """

        # Get a more forgiving version of the string
        os = text.lower().strip()

        # Cast it to the appropriate type or error
        if os == "linux":
            return "linux"
        elif os == "darwin" or os == "macos":
            return "darwin"
        else:
            raise ValueError(f"Unknown OS '{text}'")

    def is_given(self) -> bool:
        """
            Returns whether or not the OS is given manually or simply the host OS.
        """

        return self._is_given

    def is_native(self) -> bool:
        """
            Returns whether or not the current OS is native.
        """

        return self._is_native



    def to_rust(self) -> str:
        """
            Returns a string representation that makes sense for Rust.
        """

        return self._os



class Command(abc.ABC):
    """
        Baseclass for Commands, whether virtual or calling some subprocess.
    """

    @abc.abstractmethod
    def __init__(self) -> None:
        # Simply init as empty (no parent stuff)
        pass

    @abc.abstractmethod
    def __str__(self) -> str:
        """
            Allows the Command to be formatted.
        """
        pass

    @abc.abstractmethod
    def run(self, _args: argparse.Namespace) -> int:
        """
            Runs the command. Returns the 'error code', which may be some wacky
            stuff in the case of abstract commands. In any case, '0' means
            success.
        """
        pass

class ShellCommand(Command):
    """
        Command that runs some shell script.
    """

    _exec        : str
    _args        : list[str]
    _env         : dict[str, str | None]
    _description : str | None

    
    def __init__(self, exec: str, *args: str, env: dict[str, str | None] = {}, description: str | None = None) -> None:
        """
            Constructor for the Command class.

            Arguments:
            - `exec`: The executable to run.
            - `args`: An (initial) list of arguments to pass to the command.
            - `env`: The environment variables to set in the command. The values given here will overwrite or extend the default environment variables. To unset one, set it to 'None'.
            - `description`: If given, replaces the description with this. Use '$CMD' to have part of it replaced with the command string.
        """

        # Set the base stuff
        Command.__init__(self)

        # Populate ourselves, ez
        self._exec        = exec
        self._args        = list(args)
        self._env         = env
        self._description = description

    def __str__(self) -> str:
        """
            Allows the Command to be formatted.
        """

        # Compute the cmd string
        scmd = self._exec if not " " in self._exec else f"\"{self._exec}\""
        for arg in self._args:
            arg = resolve_args(arg, "release" if not args.dev else "debug", args.os.to_rust(), args.arch.to_rust())
            scmd += " " + (arg if not " " in arg else f"\"{arg}\"").replace("\\", "\\\\").replace("\"", "\\\"")

        # Compute the env string
        env = os.environ.copy()
        senv = ""
        for (name, value) in self._env.items():
            # Mark all of these, but only the changes
            if value is not None and name in env and env[name] == value: continue
            if value is None and name not in env: continue

            # Possibly replace values
            value = resolve_args(value, "release" if not args.dev else "debug", args.os.to_rust(), args.arch.to_rust())
            svalue = (value if value is not None else '<unset>').replace("\\", "\\\\").replace("\"", "\\\"")

            # Add it to the string
            if len(senv) > 0: senv += " "
            senv += "{}={}".format(name, svalue if not " " in svalue else f"\"{svalue}\"")

        # If a description, return that instead
        if self._description is not None:
            # Possible replace with the command, though
            description = self._description.replace("$CMD", scmd)
            description = self._description.replace("$ENV", senv)
            return description

        # Otherwise, just return the command
        return "{}{}".format(scmd, f" (with {senv})" if len(senv) > 0 else "")



    def add(self, *args: str) -> None:
        """
            Appends the given (string) arguments to the list of arguments.
        """

        self._args += list(args)

    def add_env(self, *args: tuple[str, str | None]) -> None:
        """
            Sets the given (string, value) pair as an environment variable for this command.

            Use a value of 'None' to unset a value in the default environment.
        """

        # Add it, overwriting junk if necessary
        for (name, value) in args:
            self._env[name] = value



    def run(self, args: argparse.Namespace) -> int:
        """
            Runs the command. Returns the 'error code', which may be some wacky
            stuff in the case of abstract commands. In any case, '0' means
            success.
        """

        # Construct the final environment
        env = os.environ.copy()
        for (name, value) in self._env.items():
            # Either insert or delete the value
            if value is not None:
                # Possibly replace values
                value = resolve_args(value, "release" if not args.dev else "debug", args.os.to_rust(), args.arch.to_rust())

                # Done
                env[name] = value
            elif name in env:
                del env[name]

        # Resolve the arguments
        args = [ resolve_args(arg, "release" if not args.dev else "debug", args.os.to_rust(), args.arch.to_rust()) for arg in self._args ]

        # Start the process
        handle = subprocess.Popen([self._exec] + args, stdout=sys.stdout, stderr=sys.stderr, env=env, cwd=os.getcwd())
        handle.wait()
        return handle.returncode

class PseudoCommand(Command):
    """
        A command that actually just runs some Python code when executed.
    """

    _desc : str
    _call : typing.Callable[[], int]


    def __init__(self, description: str, callback: typing.Callable[[], int]) -> None:
        """
            Constructor for the PseudoCommand class.

            Arguments:
            - `description`: The string to print when running this command.
            - `callback`: The code to run when the command is executed.
        """

        # Set the base stuff
        Command.__init__(self)

        # Populate ourselves, ez
        self._desc = description
        self._call = callback

    def __str__(self) -> str:
        """
            Allows the Command to be formatted.
        """

        return self._desc



    def run(self, _args: argparse.Namespace) -> int:
        """
            Runs the command. Returns the 'error code', which may be some wacky
            stuff in the case of abstract commands. In any case, '0' means
            success.
        """

        # Simply run the callback
        return self._call()





##### TARGETS #####
class Target(abc.ABC):
    """
        Virtual baseclass for all targets.
    """

    name        : str
    _deps       : list[Target]
    _weak_deps  : list[Target]
    description : str


    @abc.abstractmethod
    def __init__(self, name: str, deps: list[str], weak_deps: list[str], description: str = "") -> None:
        """
            Baseclass constructor for the Target.

            # Arguments
            - `name`: The name of the Target.
            - `deps`: A list of (strong) dependencies for the Target. If any of these need to be recompiled, then this Target will be recompiled as well.
            - `weak_deps`: A list of weak dependencies for the Target. They will be build if they have to, but a rebuild of that dependency does not (necessarily) trigger a rebuild of this Target.
            - `description`: If a non-empty string, then it's a description of the target s.t. it shows up in the list of all Targets.
        """

        self.name        = name
        self._deps       = deps
        self._weak_deps  = weak_deps
        self.description = description



    def deps(self, _args: argparse.Namespace) -> list[Target]:
        """
            Returns the dependencies of this Target.
        """

        return self._deps

    def weak_deps(self, _args: argparse.Namespace) -> list[Target]:
        """
            Returns the weak dependencies of this Target.
        """

        return self._weak_deps

    @abc.abstractmethod
    def srcs(self, _args: argparse.Namespace) -> list[str]:
        """
            Returns the list of source files upon which this Target relies.

            Specifically, if any of these source files changed, the the Target will be rebuild.
        """
        pass



    def check_regen(self, args: argparse.Namespace) -> bool:
        """
            Checks if the target needs to be rebuild for the given architecture
            and the given release mode.

            Note that this function still has to be extended per child class.
        """

        # We can always skip if certain arguments are given
        if args.force:
            pdebug(f"Recompiling '{self.name}' because of the '--force' flag")
            return True
        if args.no_compile: return False

        # We do not _check_ for regeneration, but instead build the weak dependencies
        for dep_name in self.weak_deps(args):
            build_dep(args, dep_name, set())

        # Check if any of our dependencies need regeneration
        for dep_name in self.deps(args):
            # Get the actual target behind the name
            if dep_name not in targets:
                cancel(f"Unknown target '{dep_name}'", file=sys.stderr)
            dep = targets[dep_name]

            # Check its regeneration function
            if dep.check_regen(args):
                pdebug(f"Recompiling '{self.name}' because its dependency ({dep_name}) needs to be recompiled")
                return True

        # Possibly indirect the to-be-build target to another
        target = self.redirect_build(args)

        # Then, recompile if the last time we compiled this target it was for another mode (i.e., --dev or not)
        path = args.cache + f"/flags/{target.name}"
        if not os.path.isfile(path):
            pdebug(f"Recompiling '{target.name}' because its flag cache ({path}) was not found")
            return True
        try:
            with open(path, "r") as h:
                is_dev = h.read().lower().strip()
            if args.dev != (is_dev == "true"):
                pdebug(f"Recompiling '{target.name}' because its previous compilation was with different flags (now compiling for {'release' if not args.dev else 'debug'}, compiled for {'release' if not is_dev == 'true' else 'debug'})")
                return True
        except IOError as e:
            pwarning(f"Could not read '{path}': {e} (assuming target has never been build before)")
            return True

        # Also check, if the file has any sources, whether they need to be recompiled
        for src in target.srcs(args):
            # Resolve the source
            src = resolve_args(src, "release" if args.dev else "debug", args.os.to_rust(), args.arch.to_rust())

            # Check it
            if needs_recompile(args.cache + "/hashes", src):
                pdebug(f"Recompiling '{target.name}' because one of its sources ({src}) has never been compiled or is changed")
                return True

        # If all that checks out, returns the child check
        return target.should_regenerate(args)

    @abc.abstractmethod
    def should_regenerate(self, _args: argparse.Namespace) -> bool:
        """
            Returns whether or not the child thinks the Target should regenerate on top of the parent check.
        """
        pass



    # def build_deps(self, args: argparse.Namespace, building: set[str] = set()) -> bool:
    #     """
    #         Builds all dependencies of the Target.

    #         Building this way will attempt to fix cyclic dependencies by keep track of which targets we already built.

    #         Returns whether or not anything was actually rebuild (i.e., if the actual Target has to be refreshed)
    #     """

    #     # Iterate over all of our dependencies
    #     built_something = False
    #     for dep_name in self.deps(args):
    #         # Get the actual target behind the name
    #         if dep_name not in targets:
    #             cancel(f"Unknown target '{dep_name}'", file=sys.stderr)
    #         dep = targets[dep_name]

    #         # Skip this dependency if we're already commited to building them
    #         if dep_name in building: continue

    #         # Build its dependencies first
    #         building.add(dep_name)
    #         built_something = dep.build_deps(args, building)

    #         # Now check if we need to build it
    #         if not dep.check_regen(args): continue
    #         built_something = built_something or dep.build(args)

    #     # Done
    #     return built_something

    def build(self, args: argparse.Namespace, _building: set[str] = set()):
        """
            Builds the target using the command generated by the specific
            child.

            Upon failure, will throw the appropriate commands.

            The '_building' argument is used to avoid cyclic dependencies while building dependencies.
        """

        # Compute some colour strings
        debug_start = "\033[1m" if supports_color() else ""
        error_start = "\033[31;1m" if supports_color() else ""
        end         = "\033[0m" if supports_color() else ""

        # Now only continue if we need to recompile
        if not self.check_regen(args): return

        # Otherwise, build all dependencies first
        _building.add(self.name)
        for dep_name in self.deps(args) + self.weak_deps(args):
            build_dep(args, dep_name, _building)
        # All dependencies should now be ready

        # Possibly indirect the to-be-build target to another
        target = self.redirect_build(args)

        # Get the command
        cmds = target.cmds(args)
        for cmd in cmds:
            print(f" > {debug_start}{cmd}{end}")

            # Run it
            res = cmd.run(args)
            if res != 0:
                print(f"\n{debug_start}Job '{error_start}{cmd}{end}{debug_start}' failed. See output above.{end}\n", file=sys.stderr)
                exit(1)

        # Update the flags status
        try:
            # Make the directory
            path = args.cache + "/flags"
            os.makedirs(path, exist_ok=True)

            # Write the file
            with open(path + f"/{target.name}", "w") as h:
                h.write("true" if args.dev else "false")
        except IOError as e:
            perror(f"Could not update last flags for {target.name}: {e} (will recompile every time)")

        # Update the sources for this target (if any)
        for src in target.srcs(args):
            # Resolve the source
            src = resolve_args(src, "release" if args.dev else "debug", args.os.to_rust(), args.arch.to_rust())
            # Update its cache entry
            update_cache(args.cache + "/hashes", src)

    def redirect_build(self, _args: argparse.Namespace) -> Target:
        """
            Possibly redirects the build to another target (in the case of abstract targets)
        """

        return self

    @abc.abstractmethod
    def cmds(self, _args: argparse.Namespace) -> list[Command]:
        """
            Returns the commands to run to build the target given the given
            architecture and release mode.

            Will raise errors if it somehow fails to do so.
        """
        pass



class ExtendTarget(Target):
    """
        A meta target that simply appends a list of commands to the end of some other target.
    """

    _target          : Target
    _prefix_commands : list[Command]
    _suffix_commands : list[Command]


    def __init__(self, name: str, target: Target, prefix_commands: list[Command] = [], suffix_commands: list[Command] = [], deps: list[str] = [], weak_deps: list[str] = [], help: str = "") -> None:
        """
            Constructor for the ExtendTarget class.

            Arguments:
            - `name`: The name of the target. Only used within the script to reference it later.
            - `target`: The Target to extend with additional Commands.
            - `prefix_commands`: The Commands that will be preprended before those of the given Target.
            - `suffix_commands`: The Commands that will be preprended after those of the given Target.
            - `deps`: A list of dependencies that will have to be build first before this target may be.
            - `weak_deps`: A list of weak dependencies for the Target. They will be build if they have to, but a rebuild of that dependency does not (necessarily) trigger a rebuild of this Target.
            - `help`: A string describing the target (for in '--targets').
        """

        # Call the parent constructor
        Target.__init__(self, name, deps, weak_deps, help)

        # Add the target ands commands
        self._target          = target
        self._prefix_commands = prefix_commands
        self._suffix_commands = suffix_commands



    def deps(self, _args: argparse.Namespace) -> list[Target]:
        """
            Returns the dependencies of this Target.
        """

        # Return our own deps plus that of the nested target
        return self._deps + self._target.deps(args)

    def weak_deps(self, _args: argparse.Namespace) -> list[Target]:
        """
            Returns the weak dependencies of this Target.
        """

        # Return our own deps plus that of the nested target
        return self._weak_deps + self._target.weak_deps(args)

    def srcs(self, _args: argparse.Namespace) -> list[str]:
        """
            Returns the list of source files upon which this Target relies.

            Specifically, if any of these source files changed, the the Target will be rebuild.
        """

        # Return the sources of the nexted target
        return self._target.srcs(args)



    def should_regenerate(self, _args: argparse.Namespace) -> bool:
        """
            Returns whether or not the child thinks the Target should regenerate on top of the parent check.
        """

        # Simply pass to the internal target
        return self._target.check_regen(args)



    def redirect_build(self, _args: argparse.Namespace) -> Target:
        """
            Possibly redirects the build to another target (in the case of abstract targets)
        """

        return self._target

    def cmds(self, args: argparse.Namespace) -> list[Command]:
        """
            Returns the commands to run to build the target given the given
            architecture and release mode.

            Will raise errors if it somehow fails to do so.
        """

        # First get the commands of the internal target
        cmds = self._target.cmds(args)

        # Return that with prefixes and suffixes
        return self._prefix_commands + cmds + self._suffix_commands

class EitherTarget(Target):
    """
        Defines a build target that can switch between two different targets based on some runtime property.
    """

    _targets  : map[typing.Any, Target]
    _opt_name : str


    def __init__(self, name: str, opt_name: str, targets: map[typing.Any, Target], deps: list[str] = [], weak_deps: list[str] = [], help: str = "") -> None:
        """
            Constructor for the EitherTarget class.

            Arguments:
            - `name`: The name of the target. Only used within the script to reference it later.
            - `opt_name`: The name of the argument in the arguments dict to switch on.
            - `targets`: The Value/Target mapping based on the given argument.
            - `deps`: A list of dependencies that will have to be build first before this target may be.
            - `weak_deps`: A list of weak dependencies for the Target. They will be build if they have to, but a rebuild of that dependency does not (necessarily) trigger a rebuild of this Target.
            - `help`: A string describing the target (for in '--targets').
        """

        # Set the toplevel stuff
        Target.__init__(self, name, deps, weak_deps, description=help)

        # Set the options
        self._targets  = targets
        self._opt_name = opt_name



    def deps(self, args: argparse.Namespace) -> list[Target]:
        """
            Returns the dependencies of this Target.
        """

        # Check which one based on the given set of arguments
        val = getattr(args, self._opt_name)
        if val not in self._targets:
            raise ValueError(f"Value '{val}' is not a possible target for EitherTarget '{self.name}'")

        # Return its dependencies
        return self._deps + self._targets[val].deps(args)

    def weak_deps(self, _args: argparse.Namespace) -> list[Target]:
        """
            Returns the weak dependencies of this Target.
        """

        # Check which one based on the given set of arguments
        val = getattr(args, self._opt_name)
        if val not in self._targets:
            raise ValueError(f"Value '{val}' is not a possible target for EitherTarget '{self.name}'")

        # Return our own deps plus that of the nested target
        return self._weak_deps + self._targets[val].weak_deps(args)

    def srcs(self, args: argparse.Namespace) -> list[str]:
        """
            Returns the list of source files upon which this Target relies.

            Specifically, if any of these source files changed, the the Target will be rebuild.
        """

        # Check which one based on the given set of arguments
        val = getattr(args, self._opt_name)
        if val not in self._targets:
            raise ValueError(f"Value '{val}' is not a possible target for EitherTarget '{self.name}'")

        # Return the sources of the proper nested target
        return self._targets[val].srcs(args)



    def should_regenerate(self, _args: argparse.Namespace) -> bool:
        """
            Returns whether or not the child thinks the Target should regenerate on top of the parent check.
        """

        # Check which one based on the given set of arguments
        val = getattr(args, self._opt_name)
        if val not in self._targets:
            raise ValueError(f"Value '{val}' is not a possible target for EitherTarget '{self.name}'")

        # Use that target's `check_regen()`
        return self._targets[val].check_regen(args)



    def redirect_build(self, _args: argparse.Namespace) -> Target:
        """
            Possibly redirects the build to another target (in the case of abstract targets)
        """

        # Check which one based on the given set of arguments
        val = getattr(args, self._opt_name)
        if val not in self._targets:
            raise ValueError(f"Value '{val}' is not a possible target for EitherTarget '{self.name}'")

        # Return that target
        return self._targets[val]

    def cmds(self, args: argparse.Namespace) -> list[Command]:
        """
            Returns the commands to run to build the target given the given
            architecture and release mode.

            Will raise errors if it somehow fails to do so.
        """

        # Check which one based on the given set of arguments
        val = getattr(args, self._opt_name)
        if val not in self._targets:
            raise ValueError(f"Value '{val}' is not a possible target for EitherTarget '{self.name}'")

        # Use that target's `cmds()`
        return self._targets[val].cmds(args)

class AbstractTarget(Target):
    """
        A target that launches other targets, possibly in sequence.

        It does so by using the readily available 'deps' field.
    """


    def __init__(self, name: str, deps: list[str], weak_deps: list[str] = [], help: str = "") -> None:
        """
            Constructor for the AbstractTarget class.

            Arguments:
            - `name`: The name of the target. Only used within the script to reference it later.
            - `deps`: A list of dependencies that will have to be build first before this target may be.
            - `weak_deps`: A list of weak dependencies for the Target. They will be build if they have to, but a rebuild of that dependency does not (necessarily) trigger a rebuild of this Target.
            - `help`: A string describing the target (for in '--targets').
        """

        # Simply call the parent constructor
        Target.__init__(self, name, deps, weak_deps, help)



    def srcs(self, _args: argparse.Namespace) -> list[str]:
        """
            Returns the list of source files upon which this Target relies.

            Specifically, if any of these source files changed, the the Target will be rebuild.
        """

        # Don't return any sources of this AbstractTarget specifically
        return []



    def should_regenerate(self, _args: argparse.Namespace) -> bool:
        """
            Returns whether or not the child thinks the Target should regenerate on top of the parent check.
        """

        # There's nothing to do, so nothing to regen (always)
        return False



    def cmds(self, _args: argparse.Namespace) -> list[Command]:
        """
            Returns the commands to run to build the target given the given
            architecture and release mode.

            Will raise errors if it somehow fails to do so.
        """

        # Nothing to do
        return []



class CrateTarget(Target):
    """
        Defines a build target that relies on Cargo for build caching.
    """

    src                  : list[Target]
    pkgs                 : list[str]
    target               : str | None
    target_given_default : bool
    force_dev            : bool
    env                  : dict[str, str | None]


    def __init__(self, name: str, packages: str | list[str], target: str | None = None, target_given_default: bool = False, force_dev: bool = False, env: dict[str, str | None] = {}, deps: list[str] = [], weak_deps: list[str] = [], help: str = "") -> None:
        """
            Constructor for the CrateTarget class.

            Arguments:
            - `name`: The name of the target. Only used within this script to reference it later.
            - `packages`: The list of cargo packages to build for this target. Leave empty to build the default.
            - `target`: An optional target to specify if needed. Should contain '$ARCH' which will be replaced with the desired architecture.
            - `target_given_default`: If True, does not specify '--target' in Cargo if the user did not explicitly specified so.
            - `force_dev`: If given, always builds the development binary (i.e., never adds '--release' to the Cargo command).
            - `env`: If given, overrides/adds environment variables for the build command. If set to 'None', then it unsets that environment variable instead.
            - `deps`: A list of dependencies that will have to be build first before this target may be.
            - `weak_deps`: A list of weak dependencies for the Target. They will be build if they have to, but a rebuild of that dependency does not (necessarily) trigger a rebuild of this Target.
            - `help`: A string describing the target (for in '--targets')
        """

        # Resolve the packages to a list (always)
        if type(packages) == str:
            packages = [ packages ]

        # Set the toplevel stuff
        Target.__init__(self, name, deps, weak_deps, description=help)

        # Simply set
        self.pkgs                 = packages
        self.target               = target
        self.target_given_default = target_given_default
        self.force_dev            = force_dev
        self.env                  = env



    def srcs(self, _args: argparse.Namespace) -> list[str]:
        """
            Returns the list of source files upon which this Target relies.

            Specifically, if any of these source files changed, the the Target will be rebuild.
        """

        # Don't return any sources of this target
        return []



    def should_regenerate(self, _args: argparse.Namespace) -> bool:
        """
            Returns whether or not the child thinks the Target should regenerate on top of the parent check.
        """

        # Always regenerate (we let Cargo handle it)
        pdebug(f"Recompiling '{self.name}' to let Cargo deal with the caches")
        return True



    def cmds(self, args: argparse.Namespace) -> list[Command]:
        """
            Returns the commands to run to build the target given the given
            architecture and release mode.

            Will raise errors if it somehow fails to do so.
        """

        # Start collecting the arguments for cargo
        cmd = ShellCommand("cargo", "build", env=self.env)
        if self.target is not None and (not self.target_given_default or args.arch.is_given()):
            cmd.add("--target", resolve_args(self.target, "release" if not args.dev else "debug", args.os.to_rust(), args.arch.to_rust()))
        if not self.force_dev and not args.dev:
            cmd.add("--release")
        for pkg in self.pkgs:
            cmd.add("--package", pkg)

        # Done
        return [ cmd ]

class DownloadTarget(Target):
    """
        Defines a build target that downloads a file.
    """

    _addr    : str
    _outfile : str
    _getter  : str


    def __init__(self, name: str, output: str, address: str, arch_getter: str = "__str__", deps: list[str] = [], weak_deps: list[str] = [], help: str = "") -> None:
        """
            Constructor for the DownloadTarget class.

            Arguments:
            - `name`: The name of the target. Only used within this script to reference it later.
            - `output`: The location of the downloaded file.
            - `address`: The address to pull the file from. Supports being redirected.
            - `arch_getter`: The name of the function in the Arch struct to get the appropriate string representation of the architecture for this file.
            - `deps`: A list of dependencies that will have to be build first before this target may be.
            - `weak_deps`: A list of weak dependencies for the Target. They will be build if they have to, but a rebuild of that dependency does not (necessarily) trigger a rebuild of this Target.
            - `help`: A string describing the target (for in '--targets')
        """

        # Set the toplevel stuff
        Target.__init__(self, name, deps, weak_deps, description=help)

        # Store the address and the getter
        self._addr    = address
        self._outfile = output
        self._getter  = arch_getter



    def srcs(self, _args: argparse.Namespace) -> list[str]:
        """
            Returns the list of source files upon which this Target relies.

            Specifically, if any of these source files changed, the the Target will be rebuild.
        """

        # Don't return any sources of this target
        return []



    def should_regenerate(self, _args: argparse.Namespace) -> bool:
        """
            Returns whether or not the child thinks the Target should regenerate on top of the parent check.
        """

        # Otherwise, always regenerate (we cannot check if the source is up-to-date otherwise)
        pdebug(f"Recompiling '{self.name}' because we cannot check to-be-downloaded file for updates")
        return True



    def cmds(self, args: argparse.Namespace) -> list[Command]:
        """
            Returns the commands to run to build the target given the given
            architecture and release mode.

            Will raise errors if it somehow fails to do so.
        """

        # Define the function that downloads the file
        addr    = resolve_args(self._addr, "release" if not args.dev else "debug", args.os.__str__(), args.arch.__str__())
        outfile = resolve_args(self._outfile, "release" if not args.dev else "debug", args.os.__str__(), args.arch.__str__())
        def get_file() -> int:
            s = requests.Session()

            # Run the request
            try:
                with open(outfile, "wb") as f:
                    with s.get(addr, allow_redirects=True, stream=True) as r:
                        # Make sure it succeeded
                        if r.status_code != 200:
                            cancel(f"Failed to download file: server returned exit code {r.status_code} ({http.client.responses[r.status_code]})")

                        # Iterate over the result
                        print(f"   (File size: {to_bytes(int(r.headers['Content-length']))})")
                        prgs = ProgressBar(stop=int(r.headers['Content-length']), prefix=" " * 13)
                        chunk_start = time.time()
                        for chunk in r.iter_content():
                            chunk_time = time.time() - chunk_start
                            prgs.update_prefix(f"   {to_bytes(len(chunk) * (1 / chunk_time)).rjust(10)}/s ")
                            f.write(chunk)
                            prgs.update(len(chunk))
                            chunk_start = time.time()
                        prgs.stop()

            # Catch request Errors
            except requests.exceptions.RequestException as e:
                cancel(f"Failed to download file: {e}", code=e.errno)

            # Catch IO Errors
            except IOError as e:
                cancel(f"Failed to download file: {e}", code=e.errno)

            # Catch KeyboardInterrupt
            except KeyboardInterrupt as e:
                print("\n > Rolling back file download...")
                try:
                    os.remove(outfile)
                except IOError as e:
                    perror(f"Failed to rollback file: {e}")
                    return e.errno
                raise e

            # Done
            return 0

        # Wrap the function in a command
        cmd = PseudoCommand(f"Downloading '{addr}' to '{outfile}'...", get_file)

        # Now return it + the command to make the thing executable
        return [ cmd, ShellCommand("chmod", "+x", outfile) ]

class ImageTarget(Target):
    """
        Target that builds an image according to a Dockerfile.
    """

    _dockerfile  : str
    _destination : str
    _src         : list[str]
    _context     : str
    _target      : str | None
    _build_args  : dict[str, str]


    def __init__(self, name: str, dockerfile: str, destination: str, src: list[str] = [], context: str = ".", target: str | None = None, build_args: dict[str, str] = {}, deps: list[str] = [], weak_deps: list[str] = [], help: str = ""):
        """
            Constructor for the ImageTarget.

            Arguments:
            - `name`: The name of the target. Only used within this script to reference it later.
            - `dockerfile`: The location of the Dockerfile to build the image for.
            - `destination`: The path of the resulting .tar image file. May contain special strings such as '$ARCH' or '$OS'.
            - `src`: The list of source files used to build this image, to check if the image needs to be rebuild. If left empty, will always assume it has to.
            - `context`: The folder used to resolve relative directories in the Dockerfile.
            - `target`: The Docker target to build in the Dockerfile. Will build the default target if omitted.
            - `build_args`: A list of build arguments to set when building the Dockerfile.
            - `deps`: A list of dependencies that will have to be build first before this target may be.
            - `weak_deps`: A list of weak dependencies for the Target. They will be build if they have to, but a rebuild of that dependency does not (necessarily) trigger a rebuild of this Target.
            - `help`: A string describing the target (for in '--targets')
        """

        # Set the super fields
        Target.__init__(self, name, deps, weak_deps, help)

        # Set the local fields
        self._dockerfile  = dockerfile
        self._destination = destination
        self._src         = src
        self._context     = context
        self._target      = target
        self._build_args  = build_args



    def srcs(self, _args: argparse.Namespace) -> list[str]:
        """
            Returns the list of source files upon which this Target relies.

            Specifically, if any of these source files changed, the the Target will be rebuild.
        """

        # Return the list of sources
        return self._src



    def should_regenerate(self, _args: argparse.Namespace) -> bool:
        """
            Returns whether or not the child thinks the Target should regenerate on top of the parent check.
        """

        # Resolve the destination path
        destination = resolve_args(self._destination, "release" if not args.dev else "debug", args.os.to_rust(), args.arch.to_rust())

        # Check if the target file already exists
        if not os.path.isfile(destination):
            pdebug(f"Recompiling '{self.name}' because its result ({destination}) does not exist")
            return True

        # Otherwise, no recompilation needed!
        return False



    def cmds(self, args: argparse.Namespace) -> list[Command]:
        """
            Returns the commands to run to build the target given the given
            architecture and release mode.

            Will raise errors if it somehow fails to do so.
        """

        # Resolve the destination path
        destination = resolve_args(self._destination, "release" if not args.dev else "debug", args.os.to_rust(), args.arch.to_rust())

        # Add a command for the output folder
        mkdir = ShellCommand("mkdir", "-p", f"{os.path.dirname(destination)}")

        # Construct the build command
        build = ShellCommand("docker", "build", "--output", f"type=docker,dest={destination}", "-f", self._dockerfile)
        if args.arch.is_given(): build.add("--platform", args.arch.to_docker())
        if self._target is not None: build.add("--target", self._target)
        for (name, value) in self._build_args.items():
            # Resolve the value
            value = resolve_args(value, "release" if not args.dev else "debug", args.os.to_rust(), args.arch.to_rust())
            # Add it
            build.add("--build-arg", f"{name}={value}")
        build.add(self._context)

        # Return the commands to run
        return [ mkdir, build ]

class InContainerTarget(Target):
    """
        Target that builds something in a container (e.g., OpenSSL).
    """

    _image         : str
    _src           : list[str]
    _dst           : list[str]
    _attach_stdin  : bool
    _attach_stdout : bool
    _attach_stderr : bool
    _volumes       : list[tuple[str, str]]
    _context       : str
    _command       : str


    def __init__(self, name: str, image: str, src: list[str] = [], dst: list[str] = [], attach_stdin: bool = True, attach_stdout: bool = True, attach_stderr: bool = True, volumes: list[tuple[str, str]] = [], context: str = ".", command: list[str] = [], deps: list[str] = [], weak_deps: list[str] = [], help: str = "") -> None:
        """
            Constructor for the ImageTarget.

            Arguments:
            - `name`: The name of the target. Only used within this script to reference it later.
            - `image`: The tag of the image to run.
            - `src`: A list of files that will be used by the command (to check if it needs to rebuild).
            - `dst`: A list of files that will be generated by the command (will be used to check if it needs to be rebuild).
            - `attach_stdin`: Whether or not to attach stdin to the container's stdin.
            - `attach_stdout`: Whether or not to attach stdout to the container's stdout.
            - `attach_stderr`: Whether or not to attach stderr to the container's stderr.
            - `volumes`: A list of volumes to attach to the container (using '-v', so note that the source path (the first argument) must be absolute. To help, you may use '$CWD'.).
            - `context`: The build context for the docker command.
            - `command`: A command to execute in the container (i.e., what will be put after its ENTRYPOINT if relevant).
            - `deps`: A list of dependencies that will have to be build first before this target may be.
            - `weak_deps`: A list of weak dependencies for the Target. They will be build if they have to, but a rebuild of that dependency does not (necessarily) trigger a rebuild of this Target.
            - `help`: A string describing the target (for in '--targets')
        """

        # Run the parent constructor
        Target.__init__(self, name, deps, weak_deps, help)

        # Add the source and targets
        self._image         = image
        self._src           = src
        self._dst           = dst
        self._attach_stdin  = attach_stdin
        self._attach_stdout = attach_stdout
        self._attach_stderr = attach_stderr
        self._volumes       = volumes
        self._context       = context
        self._command       = command



    def srcs(self, _args: argparse.Namespace) -> list[str]:
        """
            Returns the list of source files upon which this Target relies.

            Specifically, if any of these source files changed, the the Target will be rebuild.
        """

        # Return the list of sources
        return self._src



    def should_regenerate(self, _args: argparse.Namespace) -> bool:
        """
            Returns whether or not the child thinks the Target should regenerate on top of the parent check.
        """

        # Resolve the destination path(s)
        dst = [
            resolve_args(dst, "release" if not args.dev else "debug", args.os.to_rust(), args.arch.to_rust())
            for dst in self._dst
        ]

        # Check if any of them are missing
        for d in dst:
            if not os.path.exists(d):
                pdebug(f"Recompiling '{self.name}' because one of its results ({d}) does not exist")
                return True

        # Otherwise, no need to re-install
        return False



    def cmds(self, args: argparse.Namespace) -> list[Command]:
        """
            Returns the commands to run to build the target given the given
            architecture and release mode.

            Will raise errors if it somehow fails to do so.
        """

        # Resolve the destination path(s)
        dst = [
            resolve_args(dst, "release" if not args.dev else "debug", args.os.to_rust(), args.arch.to_rust())
            for dst in self._dst
        ]

        # Get the current working directory
        cwd = os.getcwd()

        # Get the current user ID
        handle = subprocess.Popen(["id", "-u"], text=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        (stdout, stderr) = handle.communicate()
        if handle.returncode != 0: cancel(f"Failed to get current user ID using 'id -u':\n{stderr}")
        uid = stdout.strip()

        # Get the current group ID
        handle = subprocess.Popen(["id", "-g"], text=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        (stdout, stderr) = handle.communicate()
        if handle.returncode != 0: cancel(f"Failed to get current group ID using 'id -u':\n{stderr}")
        gid = stdout.strip()



        # Prepare the command
        cmd = ShellCommand("docker", "run")
        if self._attach_stdin: cmd.add("--attach", "STDIN")
        if self._attach_stdout: cmd.add("--attach", "STDOUT")
        if self._attach_stderr: cmd.add("--attach", "STDERR")
        for (src, dst) in self._volumes:
            # Possibly replace the '$CWD' in src
            src = src.replace("$CWD", cwd)
            # Add
            cmd.add("-v", f"{src}:{dst}")
        # Add the image
        cmd.add(self._image)
        # Add any commands
        for c in self._command:
            # Do standard replacements in the command
            c = resolve_args(c, "release" if not args.dev else "debug", args.os.to_rust(), args.arch.to_rust())
            cmd.add(c)
        cmds = [ cmd ]

        # If any volumes, add the command that will restore the permissions
        for (src, _) in self._volumes:
            # Possibly replace the '$CWD' in src
            src = src.replace("$CWD", cwd)
            # Add the command
            cmds.append(ShellCommand("sudo", "chown", "-R", f"{uid}:{gid}", src, description=f"Restoring user permissions to '{src}' ($CMD)..."))

        # Done, return it
        return cmds



class InstallTarget(Target):
    """
        Target that installs something (i.e., copies it to a target system folder).
    """

    _source    : str
    _target    : str
    _need_sudo : bool


    def __init__(self, name: str, source: str, target: str, need_sudo: bool, deps: list[str] = [], weak_deps: list[str] = [], help: str = "") -> None:
        """
            Constructor for the ImageTarget.

            Arguments:
            - `name`: The name of the target. Only used within this script to reference it later.
            - `source`: The source location of the file to install. May contain special parameters such as '$ARCH'.
            - `target`: The target location of the new source file.
            - `need_sudo`: Whether or not sudo is required to perform this copy.
            - `deps`: A list of dependencies that will have to be build first before this target may be.
            - `weak_deps`: A list of weak dependencies for the Target. They will be build if they have to, but a rebuild of that dependency does not (necessarily) trigger a rebuild of this Target.
            - `help`: A string describing the target (for in '--targets')
        """

        # Run the parent constructor
        Target.__init__(self, name, deps, weak_deps, help)

        # Add the source and targets
        self._source    = source
        self._target    = target
        self._need_sudo = need_sudo



    def srcs(self, _args: argparse.Namespace) -> list[str]:
        """
            Returns the list of source files upon which this Target relies.

            Specifically, if any of these source files changed, the the Target will be rebuild.
        """

        # Resolve the source path
        source = resolve_args(self._source, "release" if not args.dev else "debug", args.os.to_rust(), args.arch.to_rust())        

        # Return it then as our only source
        return [ source ]



    def should_regenerate(self, _args: argparse.Namespace) -> bool:
        """
            Returns whether or not the child thinks the Target should regenerate on top of the parent check.
        """

        # Resolve the source and target paths
        target = resolve_args(self._target, "release" if not args.dev else "debug", args.os.to_rust(), args.arch.to_rust())

        # Check if the target file doesn't exist already
        if not os.path.exists(target):
            pdebug(f"Recompiling '{self.name}' because its result ({target}) does not exist")
            return True

        # Otherwise, no need to re-install
        return False



    def cmds(self, args: argparse.Namespace) -> list[Command]:
        """
            Returns the commands to run to build the target given the given
            architecture and release mode.

            Will raise errors if it somehow fails to do so.
        """

        # Resolve the source and target paths
        source = resolve_args(self._source, "release" if not args.dev else "debug", args.os.to_rust(), args.arch.to_rust())
        target = resolve_args(self._target, "release" if not args.dev else "debug", args.os.to_rust(), args.arch.to_rust())

        # Prepare the command
        cmd = ShellCommand("sudo" if self._need_sudo else "cp", description = f"Installing '{source}' to '{target}' ($CMD)...")
        if self._need_sudo: cmd.add("cp")
        cmd.add(source, target)

        # Done, return it
        return [ cmd ]

class InstallImageTarget(Target):
    """
        Target that installs something (i.e., copies it to a target system folder).
    """

    _source    : str
    _tag       : str


    def __init__(self, name: str, source: str, tag: str, deps: list[str] = [], weak_deps: list[str] = [], help: str = "") -> None:
        """
            Constructor for the ImageTarget.

            Arguments:
            - `name`: The name of the target. Only used within this script to reference it later.
            - `source`: The source location of the file to install. May contain special parameters such as '$ARCH'.
            - `tag`: The tag that will be assigned to the new image.
            - `deps`: A list of dependencies that will have to be build first before this target may be.
            - `weak_deps`: A list of weak dependencies for the Target. They will be build if they have to, but a rebuild of that dependency does not (necessarily) trigger a rebuild of this Target.
            - `help`: A string describing the target (for in '--targets')
        """

        # Run the parent constructor
        Target.__init__(self, name, deps, weak_deps, help)

        # Add the source and targets
        self._source    = source
        self._tag       = tag



    def srcs(self, _args: argparse.Namespace) -> list[str]:
        """
            Returns the list of source files upon which this Target relies.

            Specifically, if any of these source files changed, the the Target will be rebuild.
        """

        # Resolve the source path
        source = resolve_args(self._source, "release" if not args.dev else "debug", args.os.to_rust(), args.arch.to_rust())

        # Return it as only source
        return [ source ]



    def should_regenerate(self, _args: argparse.Namespace) -> bool:
        """
            Returns whether or not the child thinks the Target should regenerate on top of the parent check.
        """

        # Resolve the source path
        source = resolve_args(self._source, "release" if not args.dev else "debug", args.os.to_rust(), args.arch.to_rust())

        # Get the list of images already loaded in the Docker daemon
        handle = subprocess.Popen([ "docker", "image", "list" ], text=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        (stdout, stderr) = handle.communicate()
        if handle.returncode != 0:
            pwarning("WARNING: Failed to check if image already exists in Docker daemon:\n{stderr}\n(Assuming it doesn't)")
            return True

        # Examine it to find one with the target tag
        found = False
        for line in stdout.split("\n"):
            if len(line.strip()) == 0: continue

            # Split the line in its relevant parts
            tag    = None
            digest = None
            i      = 0
            for part in line.split():
                part = part.strip()
                if len(part) == 0: continue

                # Count to find the proper ones
                if i == 0: tag = part
                elif i == 2: digest = part
                i += 1

            # Make sure they are both found
            if tag is None or digest is None:
                pwarning(f"Could not split line '{line}' when examining list of images; ignoring...")
                continue

            # See if the tag matches
            if self._tag == tag:
                # It's definitely found; now make sure the digest matches too
                current_digest = get_image_digest(source)
                if current_digest[:len(digest)] != digest:
                    # Compute some colour strings
                    debug_start = "\033[1m" if supports_color() else ""
                    end         = "\033[0m" if supports_color() else ""

                    # Remove the image
                    cmd = ShellCommand("docker", "image", "rm", digest, description=f"Removing out-of-date Docker image with tag '{tag}' ({digest})...")
                    print(f" > {debug_start}{cmd}{end}")
                    res = cmd.run(args)
                    if res != 0: cancel(f"Could not remove old '{tag}' image from Docker engine; please remove it manually")

                    # Done, we have to update it
                    pdebug(f"Recompiling '{self.name}' because the image with tag '{tag}' was outdated (differing hashes)")
                else:
                    # No need to update
                    found = True
                    break
        if not found:
            pdebug(f"Recompiling '{self.name}' because the image with tag '{tag}' was not loaded in the local Docker daemon")
            return True

        # Otherwise, no need to re-install
        return False



    def cmds(self, args: argparse.Namespace) -> list[Command]:
        """
            Returns the commands to run to build the target given the given
            architecture and release mode.

            Will raise errors if it somehow fails to do so.
        """

        # Resolve the source path
        source = resolve_args(self._source, "release" if not args.dev else "debug", args.os.to_rust(), args.arch.to_rust())

        # Load the image digest
        digest = get_image_digest(source)

        # Load the image.tar into the Docker engine and tag it
        cmd = ShellCommand("docker", "load", "--input", source)
        tag = ShellCommand("docker", "tag", digest, self._tag)

        # Return them all
        return [ cmd, tag ]





##### GLOBALS #####
# Whether to print debug messages or not
debug: bool = False

# A list of deduced sources
instance_srcs = {
    f"{svc}" : deduce_toml_src_dirs(f"./brane-{svc}/Cargo.toml")
    for svc in SERVICES
}
for svc in instance_srcs:
    if instance_srcs[svc] is None: cancel(f"Could not auto-deduce '{svc}-image' dependencies")

# A list of all targets in the make file.
targets = {
    "build-image" : ImageTarget("build-image", "./contrib/images/Dockerfile.build", "./target/debug/build.tar", help="Builds the image in which some of the Brane components are build."),

    "openssl" : InContainerTarget("openssl", "brane-build", dst=OPENSSL_FILES, volumes=[("$CWD", "/build")], command=["openssl", "--arch", "$ARCH"], deps=["install-build-image"], help="Builds OpenSSL in a container to compile against when building the instance in development mode."),

    "cli" : EitherTarget("cli", "precompiled", {
        True  : DownloadTarget("cli-download", "./target/$RELEASE/brane", "https://github.com/epi-project/brane/releases/latest/download/brane-$OS-$ARCH"),
        False : CrateTarget("cli-compiled", "brane-cli", target="$ARCH-unknown-linux-musl", target_given_default=True)
    }, help = "Builds the Brane Command-Line Interface (Brane CLI). You may use '--precompiled' to download it from the internet instead."),

    "branelet" : CrateTarget("branelet", "brane-let", target="$ARCH-unknown-linux-musl", target_given_default=False, help = "Builds the Brane in-package executable, for use with the `build --init` command in the CLI."),

    "instance-binaries-dev" : ExtendTarget("instance-binaries-dev",
        CrateTarget("instance-binaries-dev-inner", [ f"brane-{svc}" for svc in SERVICES ], target="$ARCH-unknown-linux-musl", target_given_default=False, force_dev=True, env={ "OPENSSL_DIR": "$CWD/" + OPENSSL_DIR, "OPENSSL_LIB_DIR": "$CWD/" + OPENSSL_DIR + "/lib", "RUSTFLAGS": "-C link-arg=-lgcc" }),
        suffix_commands=[ ShellCommand("mkdir", "-p", "./.container-bins/$ARCH") ] + [ ShellCommand("cp", f"./target/$ARCH-unknown-linux-musl/debug/brane-{svc}", f"./.container-bins/$ARCH/brane-{svc}") for svc in SERVICES ],
        deps=["openssl"],
        help="Builds the debug binaries for the instance to be used with debug-optimised service images."
    ),
    "instance"              : AbstractTarget("instance", [ f"{svc}-image" for svc in SERVICES ], help="Builds the container images (to .tar files) that comprise the Brane instance." ),

    "install-build-image" : InstallImageTarget("install-build-image", "./target/debug/build.tar", "brane-build", deps=[ "build-image" ], help="Installs the build image by loading it into the local Docker engine"),
    "install-cli"         : InstallTarget("install-cli", "./target/$RELEASE/brane", "/usr/local/bin/brane", need_sudo=True, deps=[ "cli" ], help="Installs the CLI executable to the '/usr/local/bin' directory."),
    "install-instance"    : AbstractTarget("install-instance", [ f"install-{svc}-image" for svc in SERVICES ], help="Installs the brane instance by loading the compiled images into the local Docker engine."),
}
# Generate some really repetitive entries
for svc in SERVICES:
    # Generate the instance services build targets
    targets[f"{svc}-image"] = EitherTarget(f"{svc}-image", "dev", {
        False : ImageTarget(f"{svc}-image-release", "./Dockerfile.rls", f"./target/release/brane-{svc}.tar", target=f"brane-{svc}", src=instance_srcs[svc]),
        True  : ImageTarget(f"{svc}-image-debug", "./Dockerfile.dev", f"./target/debug/brane-{svc}.tar", target=f"brane-{svc}", build_args={ "ARCH": "$ARCH" }, src=[f"./.container-bins/$ARCH/brane-{svc}"], weak_deps=["instance-binaries-dev"]),
    }, help=f"Builds the container image for the brane-{svc} service to a .tar file. Depending on whether '--dev' is given, it either builds a full release image or a development-optimised debug image (that copies the executable from the './.container-bins' folder instead of building it in-container). The development-optimised image prevents having to rebuild every time, but also creates much larger images.")

    # Generate the install targets for the images
    targets[f"install-{svc}-image"] = InstallImageTarget(f"install-{svc}-image", f"./target/$RELEASE/brane-{svc}.tar", f"brane-{svc}", deps=[ f"{svc}-image" ], help=f"Installs the brane-{svc} image by loading it into the local Docker engine.")





##### MAIN FUNCTIONS #####
def deduce_deps(target) -> list[list[Target]]:
    """
        Builds a list of things to build and the order in which to build them
        based on the target's dependency.

        The order in which they are build is equal to that given in the list of
        dependencies per target. In this case, every entry may be seen as a
        'timestep', where every dependency adds one time offset (since it needs
        to be build before its parent).

        The resulting list has one entry per 'timestep'. In other words, the
        order of the nested list matters (and must be build front to back), but
        the order within the nested lists may be arbitrary.
    """

    # Step 1: build a tree of all dependencies involved
    # The nodes are lists (an entire row), and leaves are individual Targets

    # Recursively collect all dependencies
    





def build_target(target_name, args) -> int:
    """
        Builds a target, returning 0 if everything was succesfull.

        This function acts as the 'main' function of the script.
    """

    # Attempt to get the given target
    if target_name not in targets:
        print(f"Unknown target '{target_name}'", file=sys.stderr)
        return 1
    target = targets[target_name]

    # Rebuild it only if we have to (either by dependency or some other reason)
    if target.check_regen(args):
        target.build(args)

    # Success!
    return 0



# Actual entrypoint
if __name__ == "__main__":
    # Parse the CLI arguments
    parser = argparse.ArgumentParser()
    parser.add_argument("target", nargs="*", help="The target to build. Use '--target' to see a complete list.")
    parser.add_argument("-t", "--targets", action="store_true", help="If given, shows a list of all supported targets, then quits.")
    parser.add_argument("-s", "--sources", action="store_true", help="If given, shows the automatically deduced sources that are used to check for build staleness.")
    parser.add_argument("-d", "--dev", "--development", action="store_true", help="If given, builds the binaries and images in development mode. This adds debug symbols to binaries, enables extra debug prints and (in the case of the instance) enables an optimized, out-of-image building procedure. Will result in _much_ larger images.")
    parser.add_argument("-o", "--os", help=f"Determines the OS for which to compile. Only relevant for the Brane-CLI. By default, will be the host's OS (host OS: '{Os.host()}')")
    parser.add_argument("-a", "--arch", help=f"The target architecture for which to compile. By default, will be the host's architecture (host architecture: '{Arch.host()}')")
    parser.add_argument("-p", "--precompiled", action="store_true", help="If given, will download some binaries instead of compiling them. Specifically, downloads a CLI binary and relevant instance binaries. Ignored for other targets.")
    parser.add_argument("-f", "--force", nargs=0, action=ForceOrPrecompiled, help=f"If given, forces recompilation of all assets (regardless of whether they have been build before or not). Note that this does not clear any Cargo or Docker cache, so they might still consider your source to be cached (run `{sys.argv[0] if len(sys.argv) >= 1 else 'make.py'} clean` to clear those caches). Finally, overriddes any previous occurance of '--no-compile'.")
    parser.add_argument("-n", "--no-compile", nargs=0, action=ForceOrPrecompiled, help=f"If given, forces NO recompilation of all assets (regardless of whether they have been build before or not). Overriddes any previous occurance of '--force'.")
    parser.add_argument("-c", "--cache", default="./target/make_cache", help="The location of the cache location for file hashes and such.")
    parser.add_argument("--debug", action="store_true", help="If given, whether to print debug messages (including reasons for recompilation or not)")

    # Resolve arguments
    args = parser.parse_args()
    # Set the debug flag
    if args.debug:
        debug = True
    # Resolve the OS
    try:
        args.os = Os.new(args.os) if args.os is not None else Os.host()
    except ValueError:
        cancel(f"Unknown OS '{args.os}'")
    # Resolve the architecture
    try:
        args.arch = Arch.new(args.arch) if args.arch is not None else Arch.host()
    except ValueError:
        cancel(f"Unknown architecture '{args.arch}'")

    # Handle any pre-target arguments
    if args.targets:
        print("Supported targets:")
        for target in targets:
            print(f" - `{target}`: {targets[target].description}")
        exit(0)
    if args.sources:
        to_print = "Sources:\n"
        for target_name in targets:
            target = targets[target_name]

            # Add if present
            srcs = target.srcs(args)
            if len(srcs) > 0:
                to_print += f" - {target_name}:\n"
                for s in srcs:
                    to_print += "    - {}\n".format(f"'{s}'")

        # WRite it
        if to_print != "Sources:\n":
            print(to_print, end="\n")
        else:
            print("No targets rely on source files")

        # Done
        exit(0)

    # Make sure there is at least one target
    if len(args.target) == 0:
        print("No target specified; nothing to do.")
        exit(0)

    # Before we begin, move the current working directory to that of the file itself
    os.chdir(os.path.dirname(os.path.realpath(__file__)))

    # Call for the given targets
    for target in args.target:
        res = build_target(target, args)
        if res != 0: exit(res)
    exit(0)
