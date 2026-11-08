#!/bin/bash -e
# pi-gen stage: installs the OpenDJDecks bundle into the image.
# Usage: copy this stage into pi-gen, put opendjdecks-pi-*.tar.gz into
# files/, and add "stage-opendjdecks" after stage2 in STAGE_LIST.
BUNDLE=$(ls files/opendjdecks-pi-*.tar.gz | head -1)
tar -xzf "$BUNDLE" -C "${ROOTFS_DIR}/tmp"
DIR=$(basename "$BUNDLE" .tar.gz)
on_chroot << CHROOT
/tmp/$DIR/install.sh dj
rm -rf /tmp/$DIR
CHROOT
