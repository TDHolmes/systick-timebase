#! /usr/bin/env python3
"""
eliminate some of the boiler-plate involved in running our "on hardware"
qemu tests, which are actually examples.
"""
import os
import shlex
import subprocess


def main(verbose=False):
    our_dir = os.path.dirname(os.path.realpath(__file__))
    for example in os.listdir(os.path.join(our_dir, "examples")):
        if example.startswith("test_"):
            command = 'cargo run --example {} --release --features="embedded-hal" --target thumbv7m-none-eabi'.format(
                os.path.splitext(example)[0]
            )
            print("running `{}`".format(command))
            subprocess.run(shlex.split(command), check=True)


if __name__ == "__main__":
    import argparse

    parser = argparse.ArgumentParser()
    parser.add_argument("--verbose", "-v", action="store_true", help="verbose output")

    args = parser.parse_args()
    main(verbose=args.verbose)
