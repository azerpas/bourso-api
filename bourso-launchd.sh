#!/bin/bash

# Define your arguments here
SYMBOL="1rTCW8"
ACCOUNT="a583f3c5842c34fb00b408486ef493e0"
QUANTITY="1"
# ./bourso-cli trade order new --side buy --symbol 1rTCW8 --account a583f3c5842c34fb00b408486ef493e0 --quantity 4

# Path to the bourso-cli script
SCRIPT_PATH="/path/to/your/bourso-cli"
# e.g SCRIPT_PATH="$HOME/bourso-cli-darwin"

# Define the timestamp file path
TIMESTAMP_FILE="$HOME/.bourso-cli/last_run"

# Get the current timestamp
CURRENT_TIME=$(date +%s)

# Check if the timestamp file exists and read the last execution timestamp
if [ -f "$TIMESTAMP_FILE" ]; then
    LAST_RUN=$(cat "$TIMESTAMP_FILE")
else
    LAST_RUN=0
fi

# Time interval in seconds (1 week)
INTERVAL=$((7 * 24 * 60 * 60))

# If the time since the last run is greater than or equal to the interval, run the script
if [ $((CURRENT_TIME - LAST_RUN)) -ge $INTERVAL ]; then
    # Launch the script with the terminal window "popping up"
    osascript <<EOD
tell application "Terminal"
    activate
    tell application "System Events" to tell process "Terminal" to keystroke "n" using command down
    do script "$SCRIPT_PATH trade order new --side buy --symbol $SYMBOL --account $ACCOUNT --quantity $QUANTITY" in selected tab of the front window
end tell
EOD
    # Update the timestamp file
    echo "$CURRENT_TIME" > "$TIMESTAMP_FILE"
fi