#! /usr/bin/sh

# Needs to be updated each version bump
VERSION="0.1.6"

DOWNLOAD_LOCATION="/tmp/ouch"
INSTALLATION_LOCATION="/usr/bin/ouch"
REPO_URL="https://github.com/vrmiguel/ouch"

# Panicks script if anything fails
set -e

abort() {
    echo "error occurred, aborting." ; exit 1
}

install() {
    echo "Ouch v$VERSION."

    printf "Detected system: "
    # System detection from https://stackoverflow.com/a/27776822/9982477
    # Go there to see a full table of what `uname -s` might output
    case "$(uname -s)" in
        Linux)
            echo "Linux."
            system_suffix="-ubuntu-18.04-glibc"
        ;;

        Darwin)
            echo "Mac OS X."
            system_suffix="-macOS"
        ;;

        CYGWIN*|MINGW32*|MSYS*|MINGW*)
            echo "Windows."
            system_suffix=".exe"
        ;;

        *)
            echo "ERROR."
            echo "This script only works for installing on Linux, Mac OS and Windows."
            echo "We found '$(uname -s)' instead."
            echo ""
            echo "To install 'ouch' you can opt for other installation method"
            echo "listed at $REPO_URL"
            echo ""
            echo "If you think this is an error, please open an issue"
            exit 1
        ;;
    esac

    binary_url="https://github.com/vrmiguel/ouch/releases/download/${VERSION}/ouch${system_suffix}"

    echo ""

    if [ -f "$DOWNLOAD_LOCATION" ]; then
        echo "Reusing downloaded binary at '$DOWNLOAD_LOCATION'."
    else
        echo "Downloading binary to '$DOWNLOAD_LOCATION' with curl."
        echo "From $binary_url"
        curl -fSL $binary_url -o $DOWNLOAD_LOCATION
    fi

    echo ""

    if [ "$USER" = "root" ]; then
        echo "Detected root user, trying to copy $DOWNLOAD_LOCATION to $INSTALLATION_LOCATION."
        cp $DOWNLOAD_LOCATION $INSTALLATION_LOCATION || abort
    else
        echo "Asking for \"sudo\" permissions to finish installation."
        echo "Permission is needed to copy '$DOWNLOAD_LOCATION' to '$INSTALLATION_LOCATION'"

        sudo cp $DOWNLOAD_LOCATION $INSTALLATION_LOCATION || abort
    fi

    echo ""

    echo "Successfully installed!"
    echo "See $REPO_URL for usage instructions."
}

install
