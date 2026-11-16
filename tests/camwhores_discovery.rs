use personal_downloaders::{
    Cancel, Site, sites,
    sites::camwhores,
    transport::{SitePolicy, UrlPolicy},
};
use url::Url;

const TAG: &str = "https://www.camwhores.tv/tags/fixture-performer-000/";
const POST: &str = "https://www.camwhores.tv/videos/90000001/fixture-performer-000-2025-02-18/";
const PRIVATE_POST: &str = "https://www.camwhores.tv/videos/90000002/fixture-performer-0004/";
const MEDIA: &str = "https://www.camwhores.tv/get_file/141/00000000000000000000000000000000/90000000/90000001/90000001.mp4/?v-acctoken=signed123";
const CDN: &str =
    "https://video21.woopeedoopcmwhrs.xyz/remote_control.php?file=opaque.mp4&acctoken=signed123";

#[test]
fn tag_pages_and_signed_media_are_scoped() {
    assert_eq!(sites::normalize(Site::CamWhores, TAG).unwrap(), TAG);
    let policy = SitePolicy(Site::CamWhores);
    let page = "https://www.camwhores.tv/tags/fixture-performer-000/2/";
    assert!(policy.check(&Url::parse(TAG).unwrap(), TAG).is_ok());
    assert!(policy.check(&Url::parse(page).unwrap(), page).is_ok());
    assert!(policy.check(&Url::parse(POST).unwrap(), POST).is_ok());
    assert!(policy.check(&Url::parse(MEDIA).unwrap(), POST).is_ok());
    assert!(
        policy
            .check_redirect(&Url::parse(CDN).unwrap(), POST, &Url::parse(MEDIA).unwrap())
            .is_ok()
    );
    assert!(
        policy
            .check(&Url::parse(MEDIA).unwrap(), PRIVATE_POST)
            .is_err()
    );
    assert!(
        policy
            .check_redirect(
                &Url::parse(&CDN.replace(
                    "video21.woopeedoopcmwhrs.xyz",
                    "video21.woopeedoopcmwhrs.xyz.evil.test"
                ))
                .unwrap(),
                POST,
                &Url::parse(MEDIA).unwrap()
            )
            .is_err()
    );
}

#[tokio::test]
async fn pagination_is_complete_and_private_video_stays_pending() {
    let first = format!(
        "<link rel='canonical' href='{TAG}'><div id='list_videos_common_videos_list_items'><a href='{POST}'>public</a></div><div id='list_videos_common_videos_list_pagination'><li class='last'><a data-parameters='sort_by:post_date;from:2'>Last</a></li></div>"
    );
    let second = format!(
        "<link rel='canonical' href='{TAG}'><div id='list_videos_common_videos_list_items'><a href='{PRIVATE_POST}'>private</a></div>"
    );
    let player = format!("<h1>Public clip</h1><script>video_url: '{MEDIA}'</script>");
    let listing = camwhores::resolve(TAG, &Cancel::default(), &|url: String| {
        let first = first.clone();
        let second = second.clone();
        let player = player.clone();
        async move {
            match url.as_str() {
                TAG => Ok(first),
                "https://www.camwhores.tv/tags/fixture-performer-000/2/" => Ok(second),
                POST => Ok(player),
                PRIVATE_POST => Ok("<h1>Private video</h1>".to_owned()),
                _ => anyhow::bail!("Unexpected URL: {url}"),
            }
        }
    })
    .await
    .unwrap();
    assert_eq!(listing.files.len(), 1);
    assert_eq!(listing.files[0].url, MEDIA);
    assert_eq!(listing.pending.len(), 1);
    assert_eq!(listing.pending[0].url, PRIVATE_POST);
}
