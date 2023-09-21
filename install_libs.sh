# This install script will copy the ./api/lua directory to
# "$XDG_DATA_HOME/pinnacle", falling back to "~/.local/share/pinnacle"
# if it's not defined.

if [[ ! -d "./api/lua" ]]; then
    echo "You are not in the project's root directory."
    echo "Please cd there and rerun this install script."
    exit 0
fi

lua_api_dir="$(pwd)/api/lua"

data_dir="$XDG_DATA_HOME"

if [[ -z "$data_dir" ]]; then
    data_dir="$HOME/.local/share"
fi

# Create the dir if it doesn't exist for some reason
if [[ ! -d "$data_dir" ]]; then
    mkdir -p "$data_dir"
fi

cd "$data_dir"

if [[ ! -d "$(pwd)/pinnacle" ]]; then
    mkdir pinnacle
fi

cd pinnacle

if [[ -d "$(pwd)/lua" ]]; then
    rm -r "$(pwd)/lua"
fi

cp -r "$lua_api_dir" "$(pwd)"
