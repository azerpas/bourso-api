use crate::bourso::BASE_URL;
use anyhow::{Result, Context, bail};
use regex::Regex;

use super::virtual_pad;

pub struct BoursoWebClient {
    client: reqwest::Client,
    brs_mit_cookie: String,
    virtual_pad_ids: Vec<String>,
    token: String,
}

impl BoursoWebClient {
    pub fn new() -> BoursoWebClient {
        // create a new client
        BoursoWebClient {
            client: reqwest::Client::builder()
                .redirect(reqwest::redirect::Policy::none())
                .build().unwrap(),
            brs_mit_cookie: String::new(),
            virtual_pad_ids: Default::default(),
            token: String::new(),
        }
    }

    /// Get the cookies needed to make requests to the Bourso website as a string.
    /// 
    /// # Returns
    /// 
    /// The cookies as a string.
    fn get_cookies(&self) -> String {
        format!("brsDomainMigration=migrated; __brs_mit={};", self.brs_mit_cookie)
    }

    /// Get the headers needed to make requests to the Bourso website.
    /// 
    /// # Returns
    /// 
    /// The headers as a `reqwest::header::HeaderMap`.
    fn get_headers(&self) -> reqwest::header::HeaderMap {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::COOKIE,
            self.get_cookies().parse().unwrap(),
        );
        headers
    }

    pub async fn init_session(&mut self) -> Result<()> {
        let res = self.client
            .get(format!("{BASE_URL}/connexion/"))
            .send()
            .await?
            .text()
            .await?;

        self.brs_mit_cookie = extract_brs_mit_cookie(&res)?;

        let virtual_pad_res = self.client
            .get(format!("{BASE_URL}/connexion/clavier-virtuel?_hinclude=1"))
            .headers(self.get_headers())
            .send()
            .await?
            .text()
            .await?;

        self.token = extract_challenge_token(&virtual_pad_res)?;

        self.virtual_pad_ids = extract_data_matrix_keys(&virtual_pad_res)?
            .map(|key| key.to_string())
            .to_vec();

        Ok(())
    }
}

/// Extract the __brs_mit cookie from a string, usually the response of the `/connexion/` page.
/// 
/// # Arguments
/// 
/// * `res` - The response of the `/connexion/` page as a string.
/// 
/// # Returns
/// 
/// The __brs_mit cookie as a string.
/// 
/// # Example
/// 
/// ```
/// let res = r#"<!DOCTYPE html> \n<html>\n<head>\n    <script type="text/javascript">\n    document.cookie="__brs_mit=8e6912eb6a0268f0a2411668b8bf289f; domain=." + window.location.hostname + "; path=/; ";\n    window.location.reload();\n    </script>\n</head>\n<body>\n</body>\n</html>\n\n"#;
/// let brs_mit_cookie = extract_brs_mit_cookie(&res).unwrap();
/// assert_eq!(brs_mit_cookie, "8e6912eb6a0268f0a2411668b8bf289f");
/// ```
fn extract_brs_mit_cookie(res: &str) -> Result<String> {
    let regex = Regex::new(r"(?m)__brs_mit=(?P<brs_mit_cookie>.*?);").unwrap();
    let brs_mit_cookie = regex
        .captures(&res)
        .unwrap()
        .name("brs_mit_cookie")
        .unwrap();

    Ok(brs_mit_cookie.as_str().to_string())
}

/// Extract the challenge token from a string, usually the response of the `/connexion/clavier-virtuel?_hinclude=1` page.
/// 
/// # Arguments
/// 
/// * `res` - The response of the `/connexion/clavier-virtuel?_hinclude=1` page as a string.
/// 
/// # Returns
/// 
/// The challenge token as a string.
fn extract_challenge_token(res: &str) -> Result<String> {
    let regex = Regex::new(r#"(?m)data-matrix-random-challenge\]"\)\.val\("(?P<token>.*?)"\)"#).unwrap();
    let token = regex
        .captures(&res)
        .unwrap()
        .name("token")
        .unwrap();

    Ok(token.as_str().to_string())
}

/// Extract the data matrix keys from a string, usually the response of the `/connexion/clavier-virtuel?_hinclude=1` page.
/// 
/// # Arguments
/// 
/// * `res` - The response of the `/connexion/clavier-virtuel?_hinclude=1` page as a string.
/// 
/// # Returns
/// 
/// The data matrix keys as an array of 10 strings.
fn extract_data_matrix_keys(res: &str) -> Result<[&str; 10]> {
    let regex = Regex::new(r#"(?ms)<button.*?data-matrix-key="(?P<matrix_key>[A-Z]{3})".*?src="(?P<svg>data:image.*?)">.*?</button>"#).unwrap();
    let mut keys: [&str; 10] = Default::default();
    //let mut keys = [String::new(); 10];
    // get_number_for_svg(&svg);
    for cap in regex.captures_iter(&res) {
        let matrix_key = cap.name("matrix_key").unwrap();
        let svg = cap.name("svg").unwrap();
        let number = virtual_pad::get_number_for_svg(&svg.as_str())
            .with_context(|| format!("Could not find number for svg: {}.\nIt seems like the Bourso login page has changed, please contact an admin.", svg.as_str()))?;
        keys[number as usize] = matrix_key.as_str();
    }

    Ok(keys)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_brs_mit_cookie() {
        let res = r#"<!DOCTYPE html> \n<html>\n<head>\n    <script type="text/javascript">\n    document.cookie="__brs_mit=8e6912eb6a0268f0a2411668b8bf289f; domain=." + window.location.hostname + "; path=/; ";\n    window.location.reload();\n    </script>\n</head>\n<body>\n</body>\n</html>\n\n"#;
        let brs_mit_cookie = extract_brs_mit_cookie(&res).unwrap();
        assert_eq!(brs_mit_cookie, "8e6912eb6a0268f0a2411668b8bf289f");
    }

    #[test]
    fn test_extract_challenge_token() {
        let token = extract_challenge_token(VIRTUAL_PAD_RES).unwrap();
        assert_eq!(token, "THIS-STRING_represents0the1random__ElXSl-qJoXCKnqTBiew");
    }

    #[test]
    fn test_extract_data_matrix_keys() {
        let keys = extract_data_matrix_keys(VIRTUAL_PAD_RES).unwrap();
        assert_eq!(keys, ["WZE", "UVQ", "LGK", "TLT", "ISV", "RNI", "ANP", "UCA", "FIG", "YCL"]);
    }

    const VIRTUAL_PAD_RES: &str = r#"<div class="login-matrix">
    <div class="sr-only">
        Le bouton suivant permet d&#039;activer la vocalisation du clavier virtuel de saisie du mot de passe situé juste après.
          En activant la vocalisation, vous pouvez entendre les chiffres présents sur le clavier virtuel.
          Le clavier virtuel est composé de 2 lignes de 5 boutons, chacun correspondant à un chiffre de 0 à 9.
          Naviguez au clavier avec tabs ou les flèches pour entendre le chiffre correspondant.
          Si vous utilisez une interface tactile, vous pouvez maintenir appuyé chaque bouton pour entendre le chiffre.
    </div>

    <div class="login-a11y">
        <div class="login-a11y__switch">
            

    

<div class="c-switch c-switch--outline c-field c-field--error" data-id="switch-341374934" data-name="" data-brs-field><span id="aria-l-switch-341374934" class="u-sr-only">Activer la vocalisation</span><div class="c-switch__wrapper c-field__wrapper" data-brs-field-wrapper><input
     id="switch-341374934" type="checkbox" class="c-switch__checkbox" name="switch-341374934"    data-switch-id="switch-341374934"
    data-matrix-toggle-sound ><button
     role="checkbox" type="button" class="c-switch__button-wrapper" aria-checked="false"    aria-labelledby="aria-l-switch-341374934"
    data-switch="switch-341374934"
        ><span class="c-switch__inner"></span><span class="c-switch__button"></span></button><label  class="c-switch__label c-field__label" for="switch-341374934"><span class="c-field__label-text data-label-container" >Activer la vocalisation</span></label></div></div>        </div>
        <a href="javascript://;" class="brs-tooltip" data-selector="true" data-toggle="popover" data-placement="top"
           data-trigger="hover focus" data-content="Clavier sonore accessible
          aux clients non et malvoyants. Naviguez au clavier grâce à la touche tabulation ou, sur une interface
          tactile, en maintenant la touche appuyée. Validez la saisie de chaque chiffre avec la touche espace ou la
          touche entrée.">
            <span class="c-icon c-icon--help-helpbar"></span>
        </a>
    </div>

    <div class="sasmap"
        data-matrix data-matrix-harmony         data-matrix-random-challenge-selector="[data-matrix-random-challenge]"
                >

        <ul class="password-input">
                            <li data-matrix-list-item data-matrix-list-item-index="0">
                    <button type="button"
                            data-matrix-key="WZE"
                            class="sasmap__key"
                            >
                            <img alt="" class="sasmap__img" src="data:image/svg+xml;base64, PHN2ZyBlbmFibGUtYmFja2dyb3VuZD0ibmV3IDAgMCA0MiA0MiIgdmlld0JveD0iMCAwIDQyIDQyIiB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciPjxwYXRoIGQ9Im0yMS41IDZjNC42IDAgNi40IDQuOCA2LjQgOC45cy0xLjggOC45LTYuNCA4LjljLTQuNyAwLTYuNC00LjgtNi40LTguOXMxLjgtOC45IDYuNC04Ljl6bTAgMS40Yy0zLjYgMC00LjggNC00LjggNy42IDAgMy41IDEuMiA3LjYgNC44IDcuNnM0LjgtNCA0LjgtNy42LTEuMi03LjYtNC44LTcuNnoiIGZpbGw9IiMwMDM4ODMiLz48L3N2Zz4=">
                    </button>
                </li>
                            <li data-matrix-list-item data-matrix-list-item-index="1">
                    <button type="button"
                            data-matrix-key="YCL"
                            class="sasmap__key"
                            >
                            <img alt="" class="sasmap__img" src="data:image/svg+xml;base64, PHN2ZyBlbmFibGUtYmFja2dyb3VuZD0ibmV3IDAgMCA0MiA0MiIgdmlld0JveD0iMCAwIDQyIDQyIiB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciPjxnIGZpbGw9IiMwMDM4ODMiPjxnIGVuYWJsZS1iYWNrZ3JvdW5kPSJuZXciPjxwYXRoIGQ9Im03LjYgMzEuNy0xLjYgNS44aC0xbC0yLTcuMmgxbDEuNiA2IDEuNi02aC44bDEuNiA2IDEuNi02aDFsLTIgNy4yaC0xeiIvPjxwYXRoIGQ9Im0xOCAzNC40LTIuMyAzLjFoLTEuMWwyLjgtMy43LTIuNi0zLjVoMS4xbDIuMSAyLjkgMi4xLTIuOWgxLjFsLTIuNiAzLjUgMi44IDMuN2gtMS4xeiIvPjxwYXRoIGQ9Im0yNi42IDM0LjUtMi44LTQuMWgxbDIuMiAzLjMgMi4yLTMuM2gxbC0yLjggNC4xdjNoLS45di0zeiIvPjxwYXRoIGQ9Im0zMy4xIDM2LjggNC01LjZoLTR2LS44aDUuMnYuN2wtNCA1LjZoNC4xdi44aC01LjJ2LS43eiIvPjwvZz48cGF0aCBkPSJtMTcuNyAyMC42Yy44IDEuMSAxLjkgMS45IDMuOCAxLjkgMy44IDAgNS4xLTQgNS4xLTcuNnYtLjhjLS44IDEuMi0yLjcgMi45LTUuMSAyLjktMy4xIDAtNS42LTEuOC01LjYtNS41LjEtMi44IDIuMi01LjUgNS45LTUuNSA0LjcgMCA2LjMgNC4zIDYuMyA4LjkgMCA0LjQtMS44IDguOS02LjYgOC45LTIuMyAwLTMuNi0uOS00LjYtMi4yem00LjEtMTMuMmMtMyAwLTQuMyAyLjMtNC4zIDQuMSAwIDIuOCAxLjkgNC4yIDQuMyA0LjIgMS45IDAgMy43LTEuMiA0LjctMy0uMi0yLjMtMS40LTUuMy00LjctNS4zeiIvPjwvZz48L3N2Zz4=">
                    </button>
                </li>
                            <li data-matrix-list-item data-matrix-list-item-index="2">
                    <button type="button"
                            data-matrix-key="ANP"
                            class="sasmap__key"
                            >
                            <img alt="" class="sasmap__img" src="data:image/svg+xml;base64, PHN2ZyBlbmFibGUtYmFja2dyb3VuZD0ibmV3IDAgMCA0MiA0MiIgdmlld0JveD0iMCAwIDQyIDQyIiB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciPjxnIGZpbGw9IiMwMDM4ODMiPjxnIGVuYWJsZS1iYWNrZ3JvdW5kPSJuZXciPjxwYXRoIGQ9Im0xMy45IDMxLjYtMi40IDUuOWgtLjRsLTIuNC01Ljl2NS45aC0uOXYtNy4yaDEuM2wyLjIgNS40IDIuMi01LjRoMS4zdjcuMmgtLjl6Ii8+PHBhdGggZD0ibTE5LjUgMzEuOHY1LjdoLS45di03LjJoLjlsNC4xIDUuNnYtNS42aC45djcuMmgtLjl6Ii8+PHBhdGggZD0ibTMxLjcgMzAuMmMyLjEgMCAzLjYgMS42IDMuNiAzLjdzLTEuNCAzLjctMy42IDMuN2MtMi4xIDAtMy42LTEuNi0zLjYtMy43czEuNC0zLjcgMy42LTMuN3ptMCAuOGMtMS43IDAtMi43IDEuMi0yLjcgMi45czEgMi45IDIuNiAyLjkgMi42LTEuMiAyLjYtMi45Yy4xLTEuNy0uOS0yLjktMi41LTIuOXoiLz48L2c+PHBhdGggZD0ibTIyLjYgNmMyLjMgMCAzLjYuOSA0LjcgMi4ybC0uOSAxLjFjLS44LTEuMS0xLjktMS45LTMuOC0xLjktMy43IDAtNS4xIDMuOS01LjEgNy42di44Yy43LTEuMiAyLjctMi45IDUtMi45IDMuMSAwIDUuNiAxLjggNS42IDUuNSAwIDIuOC0yLjEgNS41LTUuOCA1LjUtNC43IDAtNi4zLTQuMy02LjMtOC45IDAtNC41IDEuOC05IDYuNi05em0tLjMgOC4yYy0xLjkgMC0zLjcgMS4yLTQuNyAzIC4yIDIuNCAxLjQgNS40IDQuNyA1LjQgMyAwIDQuMy0yLjMgNC4zLTQuMSAwLTIuOS0xLjgtNC4zLTQuMy00LjN6Ii8+PC9nPjwvc3ZnPg==">
                    </button>
                </li>
                            <li data-matrix-list-item data-matrix-list-item-index="3">
                    <button type="button"
                            data-matrix-key="LGK"
                            class="sasmap__key"
                            >
                            <img alt="" class="sasmap__img" src="data:image/svg+xml;base64, PHN2ZyBlbmFibGUtYmFja2dyb3VuZD0ibmV3IDAgMCA0MiA0MiIgdmlld0JveD0iMCAwIDQyIDQyIiB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciPjxnIGZpbGw9IiMwMDM4ODMiPjxnIGVuYWJsZS1iYWNrZ3JvdW5kPSJuZXciPjxwYXRoIGQ9Im0xMy45IDM1LjloLTMuNmwtLjYgMS42aC0xbDIuOS03LjJoMS4xbDIuOSA3LjJoLTF6bS0zLjMtLjhoM2wtMS41LTMuOXoiLz48cGF0aCBkPSJtMTguNyAzMC4zaDMuMmMxLjIgMCAyIC44IDIgMS44IDAgLjktLjYgMS41LTEuMyAxLjYuOC4xIDEuNC45IDEuNCAxLjggMCAxLjItLjggMS45LTIuMSAxLjloLTMuM3YtNy4xem0zIDMuMWMuOCAwIDEuMi0uNSAxLjItMS4yIDAtLjYtLjQtMS4yLTEuMi0xLjJoLTIuMnYyLjNoMi4yem0wIDMuM2MuOCAwIDEuMy0uNSAxLjMtMS4ycy0uNS0xLjItMS4zLTEuMmgtMi4ydjIuNWgyLjJ6Ii8+PHBhdGggZD0ibTI3LjMgMzMuOWMwLTIuMiAxLjYtMy43IDMuNy0zLjcgMS4zIDAgMi4yLjYgMi43IDEuNGwtLjguNGMtLjQtLjYtMS4yLTEtMi0xLTEuNiAwLTIuOCAxLjItMi44IDIuOXMxLjIgMi45IDIuOCAyLjljLjggMCAxLjYtLjQgMi0xbC44LjRjLS42LjgtMS41IDEuNC0yLjcgMS40LTIuMSAwLTMuNy0xLjUtMy43LTMuN3oiLz48L2c+PHBhdGggZD0ibTE1LjkgMjIuM2M1LjktNC43IDkuOC04LjEgOS44LTExLjQgMC0yLjUtMi0zLjUtMy45LTMuNS0yLjEgMC0zLjguOS00LjcgMi4zbC0xLS45YzEuMi0xLjggMy4zLTIuOCA1LjctMi44IDIuNSAwIDUuNCAxLjQgNS40IDQuOSAwIDMuOC00IDcuMy05IDExLjNoOS4xdjEuM2gtMTEuNHoiLz48L2c+PC9zdmc+">
                    </button>
                </li>
                            <li data-matrix-list-item data-matrix-list-item-index="4">
                    <button type="button"
                            data-matrix-key="TLT"
                            class="sasmap__key"
                            >
                            <img alt="" class="sasmap__img" src="data:image/svg+xml;base64, PHN2ZyBlbmFibGUtYmFja2dyb3VuZD0ibmV3IDAgMCA0MiA0MiIgdmlld0JveD0iMCAwIDQyIDQyIiB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciPjxnIGZpbGw9IiMwMDM4ODMiPjxnIGVuYWJsZS1iYWNrZ3JvdW5kPSJuZXciPjxwYXRoIGQ9Im0xMC4yIDMwLjNoMi41YzIuMiAwIDMuNyAxLjYgMy43IDMuNnMtMS41IDMuNi0zLjcgMy42aC0yLjV6bTIuNSA2LjRjMS43IDAgMi44LTEuMiAyLjgtMi44IDAtMS41LTEtMi44LTIuOC0yLjhoLTEuNnY1LjZ6Ii8+PHBhdGggZD0ibTE5LjkgMzAuM2g0Ljd2LjhoLTMuOHYyLjNoMy43di44aC0zLjd2Mi41aDMuOHYuOGgtNC43eiIvPjxwYXRoIGQ9Im0yOC4xIDMwLjNoNC43di44aC0zLjh2Mi4zaDMuN3YuOGgtMy43djMuM2gtLjl6Ii8+PC9nPjxwYXRoIGQ9Im0xNi4zIDIwLjFjMSAxLjQgMi42IDIuNCA0LjggMi40IDIuNyAwIDQuMy0xLjQgNC4zLTMuNyAwLTIuNS0yLTMuNS00LjYtMy41LS43IDAtMS4zIDAtMS42IDB2LTEuM2gxLjZjMi4zIDAgNC40LTEgNC40LTMuMyAwLTIuMS0xLjktMy4zLTQuMS0zLjMtMiAwLTMuNC44LTQuNiAyLjJsLS45LS45YzEuMi0xLjUgMy4xLTIuNyA1LjYtMi43IDMgMCA1LjYgMS42IDUuNiA0LjYgMCAyLjYtMi4yIDMuOC0zLjcgNCAxLjUuMiA0IDEuNCA0IDQuM3MtMi4xIDQuOS01LjggNC45Yy0yLjggMC00LjktMS4zLTUuOS0yLjl6Ii8+PC9nPjwvc3ZnPg==">
                    </button>
                </li>
                            <li data-matrix-list-item data-matrix-list-item-index="5">
                    <button type="button"
                            data-matrix-key="FIG"
                            class="sasmap__key"
                            >
                            <img alt="" class="sasmap__img" src="data:image/svg+xml;base64, PHN2ZyBlbmFibGUtYmFja2dyb3VuZD0ibmV3IDAgMCA0MiA0MiIgdmlld0JveD0iMCAwIDQyIDQyIiB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciPjxnIGZpbGw9IiMwMDM4ODMiPjxnIGVuYWJsZS1iYWNrZ3JvdW5kPSJuZXciPjxwYXRoIGQ9Im0xMS44IDMxLjFoLTIuM3YtLjhoNS40di44aC0yLjN2Ni40aC0uOXYtNi40eiIvPjxwYXRoIGQ9Im0xOC4zIDMwLjNoLjl2NC40YzAgMS4zLjcgMi4xIDIgMi4xczItLjggMi0yLjF2LTQuNGguOXY0LjRjMCAxLjgtMSAyLjktMi45IDIuOXMtMi45LTEuMi0yLjktMi45eiIvPjxwYXRoIGQ9Im0yNy4yIDMwLjNoMWwyLjQgNi4yIDIuNC02LjJoMWwtMi45IDcuMmgtMS4xeiIvPjwvZz48cGF0aCBkPSJtMjAuMyAxNC43Yy0yLS41LTQtMS45LTQtNC4yIDAtMy4xIDIuOC00LjUgNS42LTQuNSAyLjcgMCA1LjYgMS40IDUuNiA0LjUgMCAyLjMtMiAzLjYtNCA0LjIgMi4yLjYgNC4zIDIuMiA0LjMgNC42IDAgMi44LTIuNSA0LjYtNS44IDQuNnMtNS45LTEuOC01LjktNC42Yy0uMS0yLjUgMi00LjEgNC4yLTQuNnptMS42LjZjLTEuMS4xLTQuNCAxLjItNC40IDMuOCAwIDIuMSAyLjEgMy40IDQuNCAzLjRzNC40LTEuMyA0LjQtMy40YzAtMi42LTMuNC0zLjYtNC40LTMuOHptMC03LjljLTIuMyAwLTQuMSAxLjItNC4xIDMuMyAwIDIuNCAzLjEgMy4yIDQuMSAzLjQgMS4xLS4yIDQuMS0xIDQuMS0zLjQgMC0yLjEtMS44LTMuMy00LjEtMy4zeiIvPjwvZz48L3N2Zz4=">
                    </button>
                </li>
                            <li data-matrix-list-item data-matrix-list-item-index="6">
                    <button type="button"
                            data-matrix-key="ISV"
                            class="sasmap__key"
                            >
                            <img alt="" class="sasmap__img" src="data:image/svg+xml;base64, PHN2ZyBlbmFibGUtYmFja2dyb3VuZD0ibmV3IDAgMCA0MiA0MiIgdmlld0JveD0iMCAwIDQyIDQyIiB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciPjxnIGZpbGw9IiMwMDM4ODMiPjxnIGVuYWJsZS1iYWNrZ3JvdW5kPSJuZXciPjxwYXRoIGQ9Im0xMy42IDMwLjJjMS4zIDAgMi4yLjYgMi44IDEuM2wtLjcuNWMtLjUtLjYtMS4yLTEtMi4xLTEtMS42IDAtMi44IDEuMi0yLjggMi45czEuMiAyLjkgMi44IDIuOWMuOSAwIDEuNi0uNCAxLjktLjh2LTEuNWgtMi41di0uOGgzLjR2Mi42Yy0uNy43LTEuNiAxLjItMi44IDEuMi0yIDAtMy43LTEuNS0zLjctMy43czEuNy0zLjYgMy43LTMuNnoiLz48cGF0aCBkPSJtMjUuMSAzNC4yaC00LjJ2My4zaC0uOXYtNy4yaC45djMuMWg0LjJ2LTMuMWguOXY3LjJoLS45eiIvPjxwYXRoIGQ9Im0yOS44IDMwLjNoLjl2Ny4yaC0uOXoiLz48L2c+PHBhdGggZD0ibTIzLjYgMTguOGgtOC4ydi0xLjNsNy43LTExLjJoMnYxMS4yaDIuNXYxLjNoLTIuNXY0LjdoLTEuNXptLTYuNy0xLjNoNi43di05Ljd6Ii8+PC9nPjwvc3ZnPg==">
                    </button>
                </li>
                            <li data-matrix-list-item data-matrix-list-item-index="7">
                    <button type="button"
                            data-matrix-key="UCA"
                            class="sasmap__key"
                            >
                            <img alt="" class="sasmap__img" src="data:image/svg+xml;base64, PHN2ZyBlbmFibGUtYmFja2dyb3VuZD0ibmV3IDAgMCA0MiA0MiIgdmlld0JveD0iMCAwIDQyIDQyIiB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciPjxnIGZpbGw9IiMwMDM4ODMiPjxnIGVuYWJsZS1iYWNrZ3JvdW5kPSJuZXciPjxwYXRoIGQ9Im01IDMwLjRoMi45YzEuNCAwIDIuMiAxIDIuMiAyLjJzLS44IDIuMi0yLjIgMi4yaC0ydjIuOWgtLjl6bTIuOC44aC0xLjl2Mi44aDEuOWMuOSAwIDEuNC0uNiAxLjQtMS40cy0uNS0xLjQtMS40LTEuNHoiLz48cGF0aCBkPSJtMTkuMyAzNi43LjcuNy0uNi41LS43LS43Yy0uNS4zLTEuMi41LTEuOS41LTIuMSAwLTMuNi0xLjYtMy42LTMuN3MxLjQtMy43IDMuNi0zLjdjMi4xIDAgMy42IDEuNiAzLjYgMy43LS4xIDEuMS0uNCAyLTEuMSAyLjd6bS0xLjItLjEtMS0xLjEuNi0uNSAxIDEuMWMuNC0uNS43LTEuMi43LTIgMC0xLjctMS0yLjktMi42LTIuOXMtMi42IDEuMi0yLjYgMi45IDEgMi45IDIuNiAyLjljLjUtLjEuOS0uMiAxLjMtLjR6Ii8+PHBhdGggZD0ibTI2LjIgMzQuOGgtMS40djIuOWgtLjl2LTcuMmgyLjljMS4zIDAgMi4yLjggMi4yIDIuMiAwIDEuMy0uOSAyLTEuOSAyLjFsMS45IDIuOWgtMXptLjQtMy42aC0xLjl2Mi44aDEuOWMuOCAwIDEuNC0uNiAxLjQtMS40LjEtLjgtLjUtMS40LTEuNC0xLjR6Ii8+PHBhdGggZD0ibTMyLjcgMzUuOWMuNS41IDEuMiAxIDIuMyAxIDEuMyAwIDEuNy0uNyAxLjctMS4yIDAtLjktLjktMS4xLTEuOC0xLjQtMS4yLS4zLTIuNC0uNi0yLjQtMiAwLTEuMiAxLjEtMiAyLjUtMiAxLjEgMCAxLjkuNCAyLjUgMWwtLjcuN2MtLjUtLjYtMS4zLS45LTIuMS0uOS0uOSAwLTEuNS41LTEuNSAxLjEgMCAuNy44LjkgMS43IDEuMiAxLjIuMyAyLjUuNyAyLjUgMi4yIDAgMS0uNyAyLjEtMi42IDIuMS0xLjIgMC0yLjItLjUtMi44LTEuMXoiLz48L2c+PHBhdGggZD0ibTI0LjkgNy42aC05LjV2LTEuM2gxMS4zdjFsLTcuNCAxNi4yaC0xLjZ6Ii8+PC9nPjwvc3ZnPg==">
                    </button>
                </li>
                            <li data-matrix-list-item data-matrix-list-item-index="8">
                    <button type="button"
                            data-matrix-key="RNI"
                            class="sasmap__key"
                            >
                            <img alt="" class="sasmap__img" src="data:image/svg+xml;base64, PHN2ZyBlbmFibGUtYmFja2dyb3VuZD0ibmV3IDAgMCA0MiA0MiIgdmlld0JveD0iMCAwIDQyIDQyIiB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciPjxnIGZpbGw9IiMwMDM4ODMiPjxnIGVuYWJsZS1iYWNrZ3JvdW5kPSJuZXciPjxwYXRoIGQ9Im0xMS42IDM2LjFjLjMuNC43LjcgMS40LjcuOSAwIDEuNC0uNiAxLjQtMS41di01aC45djVjMCAxLjYtMSAyLjMtMi4zIDIuMy0uOCAwLTEuNC0uMi0xLjktLjh6Ii8+PHBhdGggZD0ibTIwLjcgMzQuMy0uNy44djIuNGgtLjl2LTcuMmguOXYzLjdsMy4yLTMuN2gxLjFsLTMgMy40IDMuMiAzLjhoLTEuMXoiLz48cGF0aCBkPSJtMjcuNyAzMC4zaC45djYuNGgzLjR2LjhoLTQuMnYtNy4yeiIvPjwvZz48cGF0aCBkPSJtMTcuNCAyMC4xYzEuMSAxLjYgMi42IDIuNSA0LjggMi41IDIuNSAwIDQuMy0xLjggNC4zLTQuMiAwLTIuNi0xLjgtNC4yLTQuMy00LjItMS42IDAtMi45LjUtNC4yIDEuN2wtMS0uNnYtOWgxMHYxLjNoLTguNXY2LjhjLjktLjggMi4zLTEuNiA0LjEtMS42IDIuOSAwIDUuNSAxLjkgNS41IDUuNSAwIDMuNC0yLjYgNS42LTUuOCA1LjYtMi45IDAtNC42LTEuMS01LjgtMi44eiIvPjwvZz48L3N2Zz4=">
                    </button>
                </li>
                            <li data-matrix-list-item data-matrix-list-item-index="9">
                    <button type="button"
                            data-matrix-key="UVQ"
                            class="sasmap__key"
                            >
                            <img alt="" class="sasmap__img" src="data:image/svg+xml;base64, PHN2ZyBlbmFibGUtYmFja2dyb3VuZD0ibmV3IDAgMCA0MiA0MiIgdmlld0JveD0iMCAwIDQyIDQyIiB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciPjxwYXRoIGQ9Im0yMC44IDguMy0yLjggMy0uOS0xIDMuOC00aDEuM3YxNy4zaC0xLjV2LTE1LjN6IiBmaWxsPSIjMDAzODgzIi8+PC9zdmc+">
                    </button>
                </li>
                    </ul>

        <script>
            $(function () {
                $("[data-matrix-random-challenge]").val("THIS-STRING_represents0the1random__ElXSl-qJoXCKnqTBiew")
            })
        </script>
    </div>
</div>

<script>
    $(function(){
        $(document).find('[data-matrix]').brsMatrix();
    });
</script>"#;
}
