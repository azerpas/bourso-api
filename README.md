# Bourso CLI

This app aims to be a simple CLI powered by *[Bourso API](./src/bourso_api/)* to log in to your [BoursoBank/Boursorama](https://www.boursorama.com) account and achieve some basic tasks.

The first goal is to be able to [DCA (Dollar Cost Average)](https://www.investopedia.com/terms/d/dollarcostaveraging.asp) on given [ETFs (Exchange Traded Funds)](https://www.investopedia.com/terms/e/etf.asp) on a regular basis with your Bourso account.

A [GitHub issue follows the progress](https://github.com/azerpas/bourso-api/issues/1).

## Usage
You can download the latest release [here](https://github.com/azerpas/bourso-api/releases).

Choose the right binary for your OS between:
- `bourso-cli-darwin.tar.gz` for MacOS. 
    - After extracting the archive, right click on the binary and select "Open" to bypass MacOS app check.
- `bourso-cli-linux.tar.gz` for Linux
- `bourso-cli.exe` for Windows

⚠️ Signing in with a different IP address than the ones you usually use will trigger a security check from Bourso. You'll have to validate the connection from your phone. A [GitHub pull request](https://github.com/azerpas/bourso-api/pull/10) is open to handle this case.

### Configuration
Save your client ID with this config command:
```
./bourso-cli config
```
The password will be asked each time you run the app to avoid storing it in a file.

### Get your accounts
```
./bourso-cli accounts
```
You'll get something like this:
```
[
    Account {
        id: "1a2953bd1a28a37bd3fe89d32986e613",
        name: "BoursoBank",
        balance: 100,
        bank_name: "BoursoBank",
        kind: Banking,
    },
    Account {
        id: "a583f3c5842c34fb00b408486ef493e0",
        name: "PEA DOE",
        balance: 1000000,
        bank_name: "BoursoBank",
        kind: Trading,
    },
]
```

### Place an order
**Make sure to have a trading account with enough balance to place the order.** Check the previous section to see how to get your account ID.

🛍️ Place a buy order for 4 shares of the ETF "1rTCW8" (AMUNDI MSCI WORLD UCITS ETF - EUR) on your account "a583f3c5842c34fb00b408486ef493e0":
```
./bourso-cli trade order new --side buy --symbol 1rTCW8 --account a583f3c5842c34fb00b408486ef493e0 --quantity 4
```

*Tip: You can get the ETF ID from the tracker URL, e.g. "AMUNDI MSCI WORLD UCITS ETF - EUR" url is https://www.boursorama.com/bourse/trackers/cours/1rTCW8/ (1rTCW8)*

## Security
This app runs locally. All outbound/inbound data is sent/received to/from BoursoBank servers **only**. Your password will not be saved locally and will be asked each time you run the app. Your client ID has to be configurated and will be saved into the app data for next usages.

## Disclaimer

This script is provided as is, without any warranty. I am not responsible for any loss of funds. Use at your own risk. I am not affiliated with BoursoBank or any other project mentioned in this repository. This is not financial advice.
