//! CamWhores tag catalogs and player declared MP4 sources.
use crate::{Cancel, File, Listing, Pending, safe_name};
use anyhow::{Context, Result, bail};
use scraper::{Html, Selector};
use sha2::{Digest, Sha256};
use std::{collections::HashSet, future::Future};
use url::Url;
const HOST: &str = "www.camwhores.tv";
fn secure(u: &Url) -> bool {
    u.scheme() == "https"
        && u.host_str() == Some(HOST)
        && u.port().is_none()
        && u.username().is_empty()
        && u.password().is_none()
        && u.fragment().is_none()
}
fn slug(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 150
        && s.bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}
fn tag(u: &Url) -> Result<String> {
    if !secure(u) || u.query().is_some() {
        bail!("Introduce una etiqueta HTTPS de CamWhores")
    };
    let p: Vec<_> = u.path().trim_matches('/').split('/').collect();
    if p.len() != 2 || p[0] != "tags" || !slug(p[1]) {
        bail!("Etiqueta CamWhores no válida")
    };
    Ok(p[1].into())
}
fn post(u: &Url) -> Result<(String, String)> {
    if !secure(u) || u.query().is_some() {
        bail!("Post CamWhores no válido")
    };
    let p: Vec<_> = u.path().trim_matches('/').split('/').collect();
    if p.len() != 3
        || p[0] != "videos"
        || p[1].parse::<u64>().ok().filter(|n| *n > 0).is_none()
        || !slug(p[2])
    {
        bail!("Post CamWhores no válido")
    };
    Ok((
        p[1].into(),
        format!("https://{HOST}/videos/{}/{}/", p[1], p[2]),
    ))
}
pub fn normalize(input: &str) -> Result<String> {
    let u = Url::parse(input)?;
    Ok(format!("https://{HOST}/tags/{}/", tag(&u)?))
}
pub fn is_page(input: &str) -> bool {
    Url::parse(input).is_ok_and(|u| {
        if !secure(&u) || u.query().is_some() {
            return false;
        }
        let p: Vec<_> = u.path().trim_matches('/').split('/').collect();
        p.len() == 3
            && p[0] == "tags"
            && slug(p[1])
            && p[2].parse::<usize>().is_ok_and(|n| (2..=100).contains(&n))
    })
}
pub fn is_post(input: &str) -> bool {
    Url::parse(input).is_ok_and(|u| post(&u).is_ok())
}
pub fn validate_media(u: &Url, p: &str) -> Result<()> {
    let (id, _) = post(&Url::parse(p)?)?;
    if u.scheme() == "https"
        && u.host_str().is_some_and(|host| {
            host.strip_prefix("video")
                .and_then(|rest| rest.strip_suffix(".woopeedoopcmwhrs.xyz"))
                .is_some_and(|number| {
                    !number.is_empty()
                        && number.len() <= 3
                        && number.bytes().all(|b| b.is_ascii_digit())
                })
        })
        && u.port().is_none()
        && u.username().is_empty()
        && u.password().is_none()
        && u.fragment().is_none()
        && u.path() == "/remote_control.php"
    {
        let pairs: std::collections::HashMap<_, _> = u.query_pairs().collect();
        if pairs.len() == 2
            && pairs.get("file").is_some_and(|value| {
                value.ends_with(".mp4")
                    && value.len() <= 512
                    && value
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
            })
            && pairs.get("acctoken").is_some_and(|value| token(value))
        {
            return Ok(());
        }
        bail!("Redirección de CamWhores no válida")
    }
    let segments: Vec<_> = u.path().trim_matches('/').split('/').collect();
    if !secure(u)
        || segments.len() != 6
        || segments[0] != "get_file"
        || segments[1].parse::<u32>().is_err()
        || segments[2].len() != 32
        || !segments[2].bytes().all(|b| b.is_ascii_hexdigit())
        || segments[3].parse::<u64>().ok() != Some(id.parse::<u64>()? / 1000 * 1000)
        || segments[4] != id
        || segments[5] != format!("{id}.mp4")
        || u.query_pairs().count() != 1
        || !u
            .query_pairs()
            .any(|(key, value)| key == "v-acctoken" && token(&value))
    {
        bail!("La fuente no coincide con el vídeo solicitado")
    }
    Ok(())
}
fn token(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 8192
        && value
            .bytes()
            .all(|b| b.is_ascii_graphic() && !matches!(b, b'<' | b'>' | b'"' | b'\''))
}
fn folder(t: &str) -> String {
    let h = format!("{:x}", Sha256::digest(t.as_bytes()));
    format!("{} [camwhores-{}]", safe_name(t), &h[..16])
}
pub fn parse_catalog(html: &str, input: &str) -> Result<Vec<(String, String)>> {
    let input = normalize(input)?;
    let owner = tag(&Url::parse(&input)?)?;
    let d = Html::parse_document(html);
    let c = d
        .select(&Selector::parse("link[rel=canonical]").unwrap())
        .next()
        .and_then(|e| e.value().attr("href"))
        .context("Falta canonical de etiqueta")?;
    if normalize(c)? != input {
        bail!("La respuesta corresponde a otra etiqueta")
    };
    let list = d
        .select(&Selector::parse("#list_videos_common_videos_list_items").unwrap())
        .next()
        .context("No se encontró el catálogo")?;
    let base = Url::parse(&input)?;
    let mut out = Vec::new();
    let mut ids = HashSet::new();
    for a in list.select(&Selector::parse("a[href]").unwrap()) {
        let u = base.join(a.value().attr("href").unwrap())?;
        if let Ok((id, url)) = post(&u)
            && ids.insert(id.clone())
        {
            out.push((id, url))
        }
    }
    if out.is_empty() {
        bail!("Etiqueta {owner} sin vídeos o catálogo no reconocido")
    };
    Ok(out)
}
pub fn parse_video(html: &str, post_url: &str, tagname: &str) -> Result<File> {
    let (id, p) = post(&Url::parse(post_url)?)?;
    let re = regex::Regex::new(r#"video_url\s*:\s*'(https://www\.camwhores\.tv/get_file/[^']+)'"#)?;
    let raw = re
        .captures(html)
        .context("No se encontró la fuente MP4 declarada")?
        .get(1)
        .unwrap()
        .as_str();
    let u = Url::parse(raw)?;
    validate_media(&u, &p)?;
    let d = Html::parse_document(html);
    let title = d
        .select(&Selector::parse("h1").unwrap())
        .next()
        .map(|e| e.text().collect::<String>())
        .unwrap_or_else(|| id.clone());
    Ok(File {
        id,
        post: p,
        url: u.to_string(),
        path: format!("{}/{}.mp4", folder(tagname), safe_name(&title)),
        ..Default::default()
    })
}
pub async fn resolve<F, Fut>(input: &str, cancel: &Cancel, fetch: &F) -> Result<Listing>
where
    F: Fn(String) -> Fut,
    Fut: Future<Output = Result<String>>,
{
    let input = normalize(input)?;
    let t = tag(&Url::parse(&input)?)?;
    let html = fetch(input.clone()).await?;
    cancel.check()?;
    let mut posts = parse_catalog(&html, &input)?;
    let document = Html::parse_document(&html);
    let pages = document
        .select(
            &Selector::parse(
                "#list_videos_common_videos_list_pagination li.last a[data-parameters]",
            )
            .unwrap(),
        )
        .next()
        .and_then(|element| element.value().attr("data-parameters"))
        .and_then(|value| value.split(';').find_map(|part| part.strip_prefix("from:")))
        .map(|value| value.parse::<usize>())
        .transpose()?
        .unwrap_or(1);
    if pages > 100 {
        bail!("Catálogo demasiado extenso")
    }
    let mut seen: HashSet<String> = posts.iter().map(|(id, _)| id.clone()).collect();
    for page in 2..=pages {
        cancel.check()?;
        let url = format!("{input}{page}/");
        let body = fetch(url).await?;
        let found = parse_catalog(&body, &input)?;
        let previous = posts.len();
        posts.extend(found.into_iter().filter(|(id, _)| seen.insert(id.clone())));
        if posts.len() == previous {
            bail!("Página {page} repetida; catálogo incompleto")
        }
    }
    let mut listing = Listing {
        title: folder(&t),
        ..Default::default()
    };
    for (_, url) in posts {
        cancel.check()?;
        match fetch(url.clone())
            .await
            .and_then(|h| parse_video(&h, &url, &t))
        {
            Ok(f) => listing.files.push(f),
            Err(e) => listing.pending.push(Pending {
                url,
                error: e.to_string(),
            }),
        }
    }
    Ok(listing)
}
