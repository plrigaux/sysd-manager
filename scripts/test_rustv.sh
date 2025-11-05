#!/bin/bash

# Function to get the current Rust compiler version
get_rust_version() {
    local version_string
    version_string=$(rustc --version 2>/dev/null)
    if [ $? -ne 0 ]; then
        echo "Error: rustc command not found or failed." >&2
        return 1
    fi
    # Extract the version number (e.g., "1.70.0")
    echo "$version_string" | grep -oP 'rustc \K[0-9]+\.[0-9]+\.[0-9]+'
}

# Function to compare two version strings
# Returns 0 if version1 >= version2, 1 otherwise
compare_versions() {
    local v1=$1
    local v2=$2

    # Use sort -V for version-aware comparison
    if printf '%s\n' "$v1" "$v2" | sort -V -C; then
        return 0 # v1 is greater than or equal to v2
    else
        return 1 # v1 is less than v2
    fi
}


compare_versions2() {
    version1="1.91.0"
    version2="1.89"

    if [[ "$(printf '%s\n' "$version1" "$version2" | sort -V | head -n1)" == "$version1" && "$version1" != "$version2" ]]; then
        echo "$version1 is older than $version2"
    elif [[ "$(printf '%s\n' "$version1" "$version2" | sort -V | head -n1)" == "$version2" && "$version1" != "$version2" ]]; then
        echo "$version2 is older than $version1"
    else
        echo "$version1 and $version2 are equal"
    fi
}

# Main script logic
#required_version="1.75.0"

TOML_FILE="../Cargo.toml"

# Grep for the first occurrence of 'version', remove quotes, and cut the value
required_version=$(grep -m 1 'rust-version' "$TOML_FILE" | tr -s ' ' | tr -d \"\' | cut -d' ' -f3)


current_version=$(get_rust_version)

if [ $? -ne 0 ]; then
    exit 1 # Exit if rustc command failed
fi

if [ -z "$current_version" ]; then
    echo "Could not determine current Rust version." >&2
    exit 1
fi

echo "Current Rust version: $current_version"
echo "Required Rust version: $required_version"

if compare_versions "$current_version" "$required_version"; then
    echo "Rust version meets or exceeds the requirement."
else
    echo "Rust version is older than the required version. Please update."
fi

compare_versions2

