## How to capture

Use this script https://github.com/Yamakaky/joycontrol/blob/master/scripts/relay_joycon.py.
`python -m venv` is useful to not have to install everything on `/usr`.

Example:

`sudo python scripts/relay_joycon.py -l /tmp/bt.log`

Then use this command for next runs:

`sudo python scripts/relay_joycon.py -r <BT MAC address switch> -l /tmp/bt.log`

It can take some time and doesn't always work so try restarting it a few times.

## How to parse

Send a capture file to stdin of `joytk decode`. For example to filter out the
standard input reports and show less output for subcommand requests and
replies:

```bash
cat trace/homescreen.log \
  | cargo run --bin joytk decode \
  | grep -vw '(StandardFull|RumbleOnly)' \
  | sed -re 's/.*(subcommand_reply|subcmd): //'
```