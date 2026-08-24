pub enum WFBackgroundTask {
    LoadFile,
    LoadFileNoDialog,
    ChangeDecodedAudioChannel(isize)
}