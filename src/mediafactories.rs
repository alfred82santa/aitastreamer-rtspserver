use derive_more::{Display, Error};
pub use internal::{MediaFactory, MediaFactoryManager};

#[derive(Debug, Display, Error)]
struct DefError;

trait KeyStr<T> {
    fn get<'a>(&'a self, path: &str) -> Result<&'a T, String>;

    fn has(&self, path: &str) -> bool;
}

trait KeyString<T> {
    fn get<'a>(&'a self, path: &String) -> Result<&'a T, String>;

    fn has(&self, path: &String) -> bool;
}

pub mod internal {
    use gstreamer_rtsp_server::prelude::*;
    use gstreamer_rtsp_server::{RTSPMediaFactory, RTSPMountPoints};
    use serde::de::{self, MapAccess, Visitor};
    use serde::ser::SerializeStruct;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use serde_with::{serde_as, KeyValueMap};
    use std::fmt;

    use super::{KeyStr, KeyString};

    #[serde_as]
    pub struct MediaFactory {
        media_factory: RTSPMediaFactory,

        path: String,
    }

    impl Clone for MediaFactory {
        fn clone(&self) -> Self {
            MediaFactory {
                media_factory: self.media_factory.clone(),
                path: self.path.clone(),
            }
        }
    }

    impl Serialize for MediaFactory {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            // 2 is the number of fields in the struct.
            let mut state = serializer.serialize_struct("MediaFactory", 2)?;
            state.serialize_field("launchPipeline", &self.get_launch_pipeline())?;
            state.serialize_field("shared", &self.get_shared())?;
            state.serialize_field("$key$", &self.get_path())?;
            state.end()
        }
    }

    impl<'de> Deserialize<'de> for MediaFactory {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            #[derive(Deserialize)]
            #[serde(field_identifier, rename_all = "camelCase")]
            enum Field {
                #[serde(rename = "$key$", alias = "path")]
                Path,
                Shared,
                LaunchPipeline,
            }

            struct MediaFactoryVisitor;

            impl<'de> Visitor<'de> for MediaFactoryVisitor {
                type Value = MediaFactory;

                fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                    formatter.write_str("struct MediaFactory")
                }

                fn visit_map<V>(self, mut map: V) -> Result<MediaFactory, V::Error>
                where
                    V: MapAccess<'de>,
                {
                    let mut path = None;
                    let mut launch_pipeline = None;
                    let mut shared = None;
                    while let Some(key) = map.next_key()? {
                        match key {
                            Field::Path => {
                                if path.is_some() {
                                    return Err(de::Error::duplicate_field("path"));
                                }
                                path = Some(map.next_value()?);
                            }
                            Field::LaunchPipeline => {
                                if launch_pipeline.is_some() {
                                    return Err(de::Error::duplicate_field("launchPipeline"));
                                }
                                launch_pipeline = Some(map.next_value()?);
                            }
                            Field::Shared => {
                                if shared.is_some() {
                                    return Err(de::Error::duplicate_field("shared"));
                                }
                                shared = Some(map.next_value()?);
                            }
                        }
                    }
                    let path: &str = path.ok_or_else(|| de::Error::missing_field("path"))?;
                    let launch_pipeline: &str = launch_pipeline
                        .ok_or_else(|| de::Error::missing_field("launchPipeline"))?;
                    let shared: bool = shared.or(Some(false)).unwrap();
                    Ok(MediaFactory::new(path, launch_pipeline, shared))
                }
            }

            const FIELDS: &'static [&'static str] = &["path", "launchPipeline", "shared"];
            deserializer.deserialize_struct("MediaFactory", FIELDS, MediaFactoryVisitor)
        }
    }

    impl MediaFactory {
        pub fn new(path: &str, launch_pipeline: &str, shared: bool) -> Self {
            let factory = RTSPMediaFactory::new();
            factory.set_launch(launch_pipeline);
            factory.set_shared(shared);

            MediaFactory {
                media_factory: factory,
                path: String::from(path),
            }
        }

        pub fn get_path(&self) -> &str {
            self.path.as_str()
        }

        pub fn get_shared(&self) -> bool {
            self.media_factory.is_shared()
        }

        pub fn set_shared(&self, value: bool) {
            self.media_factory.set_shared(value)
        }

        pub fn get_launch_pipeline(&self) -> String {
            String::from(self.media_factory.launch().unwrap())
        }

        pub fn set_launch_pipeline(&self, value: &str) {
            self.media_factory.set_launch(value)
        }
    }

    #[serde_as]
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct MediaFactoryManager {
        #[serde_as(as = "KeyValueMap<_>")]
        mount_points: Vec<MediaFactory>,

        #[serde(skip)]
        rtsp_mount_points: RTSPMountPoints,
    }

    impl KeyString<MediaFactory> for MediaFactoryManager {
        fn get<'a>(&'a self, path: &String) -> Result<&'a MediaFactory, String> {
            self.mount_points
                .iter()
                .filter(|mp| mp.path.eq(path))
                .next()
                .ok_or(String::from("Path does not exists"))
        }

        fn has(&self, path: &String) -> bool {
            KeyString::get(self, path).is_ok()
        }
    }

    impl KeyStr<MediaFactory> for MediaFactoryManager {
        fn get<'a>(&'a self, path: &str) -> Result<&'a MediaFactory, String> {
            KeyString::get(self, &String::from(path))
        }

        fn has(&self, path: &str) -> bool {
            KeyString::has(self, &String::from(path))
        }
    }

    impl From<RTSPMountPoints> for MediaFactoryManager {
        fn from(value: RTSPMountPoints) -> Self {
            Self {
                mount_points: vec![],
                rtsp_mount_points: value,
            }
        }
    }

    impl MediaFactoryManager {
        pub fn new(rstp_mount_points: RTSPMountPoints) -> Self {
            MediaFactoryManager {
                mount_points: vec![],
                rtsp_mount_points: rstp_mount_points,
            }
        }

        pub fn new_from_yaml_str(
            rstp_mount_points: RTSPMountPoints,
            serialized_data: &str,
        ) -> Self {
            #[serde_as]
            #[derive(Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Wrapper {
                #[serde_as(as = "KeyValueMap<_>")]
                mount_points: Vec<MediaFactory>,
            }

            print!("New config:\n{}\n", serialized_data);
            let data: Wrapper = serde_yaml::from_str(serialized_data).unwrap();

            let mut manger = Self::new(rstp_mount_points);

            data.mount_points.iter().for_each(|mp| {
                manger.add(mp.clone()).unwrap();
            });

            return manger;
        }

        pub fn add(&mut self, mount_point: MediaFactory) -> Result<&MediaFactory, String> {
            let path = String::from(mount_point.get_path());
            match KeyString::get(self, &path) {
                Ok(_mp) => Err(String::from("Already exists path")),
                _ => {
                    self.mount_points.push(mount_point);
                    let result = KeyString::get(self, &path).inspect(|mp| {
                        self.rtsp_mount_points
                            .add_factory(mp.get_path(), mp.media_factory.clone())
                    });
                    print!("New mount point: {}\n", result.as_ref().unwrap().get_path());
                    print!(
                        "Pipeline: {}\n",
                        result.as_ref().unwrap().get_launch_pipeline()
                    );

                    return result;
                }
            }
        }

        pub fn remove(&mut self, mount_point: MediaFactory) -> Result<bool, String> {
            match KeyStr::get(self, mount_point.get_path()) {
                Ok(_) => {
                    (&mut self.mount_points).retain(|item| !item.path.eq(mount_point.get_path()));
                    self.rtsp_mount_points
                        .remove_factory(mount_point.get_path());
                    Ok(true)
                }
                _ => Err(String::from("Path does not exist")),
            }
        }
    }
}
