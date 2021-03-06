/*
* Rust-FMOD - Copyright (c) 2014 Gomez Guillaume.
*
* The Original software, FmodEx library, is provided by FIRELIGHT TECHNOLOGIES.
*
* This software is provided 'as-is', without any express or implied warranty.
* In no event will the authors be held liable for any damages arising from
* the use of this software.
*
* Permission is granted to anyone to use this software for any purpose,
* including commercial applications, and to alter it and redistribute it
* freely, subject to the following restrictions:
*
* 1. The origin of this software must not be misrepresented; you must not claim
*    that you wrote the original software. If you use this software in a product,
*    an acknowledgment in the product documentation would be appreciated but is
*    not required.
*
* 2. Altered source versions must be plainly marked as such, and must not be
*    misrepresented as being the original software.
*
* 3. This notice may not be removed or altered from any source distribution.
*/

use enums::*;
use types::*;
use libc::{c_int, c_void, c_uint, c_char, c_ushort, c_void};
use ffi;
use channel;
use channel::Channel;
use sound_group;
use fmod_sys;
use std::io::timer::sleep;
use vector;
use fmod_sys;
use fmod_sys::{FmodMemoryUsageDetails, FmodSys};
use std::mem::transmute;
use std::io::File;
use std::mem;
use std::io::BufferedWriter;
use std::slice;
use std::default::Default;

struct RiffChunk {
    id: [c_char, ..4],
    size: c_int
}

struct FmtChunk {
    chunk: RiffChunk,
    w_format_tag: c_ushort,     /* format type  */
    n_channels: c_ushort,       /* number of channels (i.e. mono, stereo...)  */
    n_samples_per_sec: c_uint,  /* sample rate  */
    n_avg_bytes_per_sec: c_uint,/* for buffer estimation  */
    n_block_align: c_ushort,    /* block size of data  */
    w_bits_per_sample: c_ushort /* number of bits per sample of mono data */
}

struct DataChunk {
    chunk: RiffChunk
}

struct WavHeader {
    chunk: RiffChunk,
    riff_type: [c_char, ..4]
}

/// Wrapper for SyncPoint object
pub struct FmodSyncPoint {
    sync_point: *mut ffi::FMOD_SYNCPOINT
}

impl FmodSyncPoint {
    fn from_ptr(pointer: *mut ffi::FMOD_SYNCPOINT) -> FmodSyncPoint {
        FmodSyncPoint{sync_point: pointer}
    }
}

/// Structure describing a piece of tag data.
pub struct FmodTag {
    /// [r] The type of this tag.
    pub _type    : fmod::TagType,
    /// [r] The type of data that this tag contains
    pub data_type: fmod::TagDataType,
    /// [r] The name of this tag i.e. "TITLE", "ARTIST" etc.
    pub name     : String,
    /// [r] Pointer to the tag data - its format is determined by the datatype member
    data         : *mut c_void,
    /// [r] Length of the data contained in this tag
    data_len     : c_uint,
    /// [r] True if this tag has been updated since last being accessed with [`Sound::get_tag`](struct.Sound.html#method.get_tag)
    pub updated  : bool
}

impl Default for FmodTag {
    fn default() -> FmodTag {
        FmodTag {
            _type: fmod::TagTypeUnknown,
            data_type: fmod::TagDataTypeBinary,
            name: String::new(),
            data: ::std::ptr::mut_null(),
            data_len: 0u32,
            updated: false
        }
    }
}

impl FmodTag {
    fn from_ptr(pointer: ffi::FMOD_TAG) -> FmodTag {
        FmodTag{
            _type: pointer._type,
            data_type: pointer.datatype,
            name: {
                if pointer.name.is_not_null() {
                    unsafe {::std::str::raw::from_c_str(pointer.name as *const c_char).clone() }
                } else {
                    String::new()
                }
            },
            data: pointer.data, data_len: pointer.datalen, updated: {
                if pointer.updated == 1 {
                    true
                } else {
                    false
                }
            }
        }
    }

    fn convert_to_c(&self) -> ffi::FMOD_TAG {
        let tmp = self.name.clone();

        ffi::FMOD_TAG{
            _type: self._type,
            datatype: self.data_type,
            name: tmp.with_c_str(|c_name|{c_name as *mut c_char}),
            data: self.data,
            datalen: self.data_len,
            updated: {
                if self.updated == true {
                    1
                } else {
                    0
                }
            }
        }
    }
}

/// Sound object
pub struct Sound {
    sound: *mut ffi::FMOD_SOUND,
    can_be_deleted: bool,
    user_data: ffi::SoundData
}

pub fn get_fffi<'r>(sound: &'r mut Sound) -> &'r mut *mut ffi::FMOD_SOUND {
    &mut sound.sound
}

pub fn get_ffi(sound: &Sound) -> *mut ffi::FMOD_SOUND {
    sound.sound
}

pub fn from_ptr(sound: *mut ffi::FMOD_SOUND) -> Sound {
    Sound{sound: sound, can_be_deleted: false, user_data: ffi::SoundData::new()}
}

pub fn from_ptr_first(sound: *mut ffi::FMOD_SOUND) -> Sound {
    Sound{sound: sound, can_be_deleted: true, user_data: ffi::SoundData::new()}
}

pub fn get_user_data<'r>(sound: &'r mut Sound) -> &'r mut ffi::SoundData {
    &mut sound.user_data
}

impl Drop for Sound {
    fn drop(&mut self) {
        self.release();
    }
}

impl Sound {
    pub fn get_system_object(&self) -> Result<FmodSys, fmod::Result> {
        let mut system = ::std::ptr::mut_null();

        match unsafe { ffi::FMOD_Sound_GetSystemObject(self.sound, &mut system) } {
            fmod::Ok => Ok(fmod_sys::from_ptr(system)),
            e => Err(e)
        }
    }

    pub fn release(&mut self) -> fmod::Result {
        if self.can_be_deleted && self.sound.is_not_null() {
            match unsafe { ffi::FMOD_Sound_Release(self.sound) } {
                fmod::Ok => {
                    self.sound = ::std::ptr::mut_null();
                    fmod::Ok
                }
                e => e
            }
        } else {
            fmod::Ok
        }
    }

    pub fn play(&self) -> Result<channel::Channel, fmod::Result> {
        let mut channel = ::std::ptr::mut_null();

        match match self.get_system_object() {
            Ok(s) => { 
                unsafe { ffi::FMOD_System_PlaySound(fmod_sys::get_ffi(&s), fmod::ChannelFree, self.sound, 0, &mut channel) }
            }
            Err(e) => e
        } {
            fmod::Ok => Ok(channel::from_ptr(channel)),
            e => Err(e)
        }
    }

    pub fn play_with_parameters(&self, paused: bool, channel: &mut channel::Channel) -> fmod::Result {
        let mut chan = channel::get_ffi(channel);
        
        match self.get_system_object() {
            Ok(s) => { 
                unsafe { ffi::FMOD_System_PlaySound(fmod_sys::get_ffi(&s), fmod::ChannelReUse, self.sound, match paused {
                    true => 1,
                    false => 0
                }, &mut chan) }
            }
            Err(e) => e
        }
    }

    pub fn play_to_the_end(&self) -> fmod::Result {
        match self.play() {
            Ok(mut chan) => {
                loop {
                    match chan.is_playing() {
                        Ok(b) => {
                            if b == true {
                                sleep(30)
                            } else {
                                break;
                            }
                        },
                        Err(e) => return e,
                    }
                }
                chan.release();
                fmod::Ok
            }
            Err(err) => err,
        }
    }

    pub fn set_defaults(&self, frequency: f32, volume: f32, pan: f32, priority: i32) -> fmod::Result {
        unsafe { ffi::FMOD_Sound_SetDefaults(self.sound, frequency, volume, pan, priority) }
    }

    pub fn get_defaults(&self) -> Result<(f32, f32, f32, i32), fmod::Result> {
        let mut frequency = 0f32;
        let mut volume = 0f32;
        let mut pan = 0f32;
        let mut priority = 0i32;

        match unsafe { ffi::FMOD_Sound_GetDefaults(self.sound, &mut frequency, &mut volume, &mut pan, &mut priority) } {
            fmod::Ok => Ok((frequency, volume, pan, priority)),
            e => Err(e)
        }
    }

    pub fn set_variations(&self, frequency_var: f32, volume_var: f32, pan_var: f32) -> fmod::Result {
        unsafe { ffi::FMOD_Sound_SetVariations(self.sound, frequency_var, volume_var, pan_var) }
    }

    pub fn get_variations(&self) -> Result<(f32, f32, f32), fmod::Result> {
        let mut frequency_var = 0f32;
        let mut volume_var = 0f32;
        let mut pan_var = 0f32;

        match unsafe { ffi::FMOD_Sound_GetVariations(self.sound, &mut frequency_var, &mut volume_var, &mut pan_var) } {
            fmod::Ok => Ok((frequency_var, volume_var, pan_var)),
            e => Err(e)
        }
    }

    pub fn set_3D_min_max_distance(&self, min: f32, max: f32) -> fmod::Result {
        unsafe { ffi::FMOD_Sound_Set3DMinMaxDistance(self.sound, min, max) }
    }

    pub fn get_3D_min_max_distance(&self) -> Result<(f32, f32), fmod::Result> {
        let mut max = 0f32;
        let mut min = 0f32;

        match unsafe { ffi::FMOD_Sound_Get3DMinMaxDistance(self.sound, &mut min, &mut max) } {
            fmod::Ok => Ok((min, max)),
            e => Err(e)
        }
    }

    pub fn set_3D_cone_settings(&self, inside_cone_angle: f32, outside_cone_angle: f32, outside_volume: f32) -> fmod::Result {
        unsafe { ffi::FMOD_Sound_Set3DConeSettings(self.sound, inside_cone_angle, outside_cone_angle, outside_volume) }
    }

    pub fn get_3D_cone_settings(&self) -> Result<(f32, f32, f32), fmod::Result> {
        let mut inside_cone_angle = 0f32;
        let mut outside_cone_angle = 0f32;
        let mut outside_volume = 0f32;

        match unsafe { ffi::FMOD_Sound_Get3DConeSettings(self.sound, &mut inside_cone_angle, &mut outside_cone_angle, &mut outside_volume) } {
            fmod::Ok => Ok((inside_cone_angle, outside_cone_angle, outside_volume)),
            e => Err(e)
        }
    }

    pub fn set_3D_custom_rolloff(&self, points: Vec<vector::FmodVector>) -> fmod::Result {
        let mut points_vec = Vec::with_capacity(points.len());

        for tmp in points.move_iter() {
            points_vec.push(vector::get_ffi(&tmp));
        }
        unsafe { ffi::FMOD_Sound_Set3DCustomRolloff(self.sound, points_vec.as_mut_ptr(), points_vec.len() as i32) }
    }

    //to test
    pub fn get_3D_custom_rolloff(&self, num_points: u32) -> Result<Vec<vector::FmodVector>, fmod::Result> {
        let mut points_vec = Vec::with_capacity(num_points as uint);
        let mut pointer = points_vec.as_mut_ptr();

        match unsafe { ffi::FMOD_Sound_Get3DCustomRolloff(self.sound, &mut pointer, num_points as i32) } {
            fmod::Ok => {
                let mut points = Vec::with_capacity(points_vec.len());

                for tmp in points_vec.move_iter() {
                    points.push(vector::from_ptr(tmp));
                }
                Ok(points)
            }
            e => Err(e)
        }
    }

    pub fn set_sub_sound(&self, index: i32, sub_sound: Sound) -> fmod::Result {
        unsafe { ffi::FMOD_Sound_SetSubSound(self.sound, index, sub_sound.sound) }
    }

    pub fn get_sub_sound(&self, index: i32) -> Result<Sound, fmod::Result> {
        let mut sub_sound = ::std::ptr::mut_null();

        match unsafe { ffi::FMOD_Sound_GetSubSound(self.sound, index, &mut sub_sound) } {
            fmod::Ok => Ok(from_ptr(sub_sound)),
            e => Err(e)
        }
    }

    pub fn get_name(&self, name_len: u32) -> Result<String, fmod::Result> {
        let name = String::with_capacity(name_len as uint).into_string();

        name.with_c_str(|c_name|{
            match unsafe { ffi::FMOD_Sound_GetName(self.sound, c_name as *mut c_char, name_len as i32) } {
                fmod::Ok => Ok(unsafe {::std::str::raw::from_c_str(c_name).clone() }),
                e => Err(e)
            }
        })
    }

    pub fn get_length(&self, FmodTimeUnit(length_type): FmodTimeUnit) -> Result<u32, fmod::Result> {
        let mut length = 0u32;

        match unsafe { ffi::FMOD_Sound_GetLength(self.sound, &mut length, length_type) } {
            fmod::Ok => Ok(length),
            e => Err(e)
        }
    }

    pub fn get_format(&self) -> Result<(fmod::SoundType, fmod::SoundFormat, i32, i32), fmod::Result> {
        let mut _type = fmod::SoundTypeUnknown;
        let mut format = fmod::SoundFormatNone;
        let mut channels = 0i32;
        let mut bits = 0i32;

        match unsafe { ffi::FMOD_Sound_GetFormat(self.sound, &mut _type, &mut format, &mut channels, &mut bits) } {
            fmod::Ok => Ok((_type, format, channels, bits)),
            e => Err(e)
        }
    }

    pub fn get_num_sub_sounds(&self) -> Result<i32, fmod::Result> {
        let mut num_sub_sound = 0i32;

        match unsafe { ffi::FMOD_Sound_GetNumSubSounds(self.sound, &mut num_sub_sound) } {
            fmod::Ok => Ok(num_sub_sound),
            e => Err(e)
        }
    }

    pub fn get_num_tags(&self) -> Result<(i32, i32), fmod::Result> {
        let mut num_tags = 0i32;
        let mut num_tags_updated = 0i32;

        match unsafe { ffi::FMOD_Sound_GetNumTags(self.sound, &mut num_tags, &mut num_tags_updated) } {
            fmod::Ok => Ok((num_tags, num_tags_updated)),
            e => Err(e)
        }
    }

    //to test if tag's data needs to be filled by user
    pub fn get_tag(&self, name: String, index: i32) -> Result<FmodTag, fmod::Result> {
        let mut tag = ffi::FMOD_TAG{_type: fmod::TagTypeUnknown, datatype: fmod::TagDataTypeBinary, name: ::std::ptr::mut_null(),
            data: ::std::ptr::mut_null(), datalen: 0, updated: 0};

        match unsafe { ffi::FMOD_Sound_GetTag(self.sound, name.into_string().with_c_str(|c_name|{c_name}), index, &mut tag) } {
            fmod::Ok => Ok(FmodTag::from_ptr(tag)),
            e => Err(e)
        }
    }

    pub fn get_open_state(&self) -> Result<(fmod::OpenState, u32, bool, bool), fmod::Result> {
        let mut open_state = fmod::OpenStateReady;
        let mut percent_buffered = 0u32;
        let mut starving = 0;
        let mut disk_busy = 0;

        match unsafe { ffi::FMOD_Sound_GetOpenState(self.sound, &mut open_state, &mut percent_buffered, &mut starving, &mut disk_busy) } {
            fmod::Ok => Ok((open_state, percent_buffered, if starving == 1 {
                            true
                            } else {
                                false
                            }, if disk_busy == 1 {
                                true
                            } else {
                                false
                            })),
            e => Err(e)
        }
    }

    pub fn set_sound_group(&self, sound_group: sound_group::SoundGroup) -> fmod::Result {
        unsafe { ffi::FMOD_Sound_SetSoundGroup(self.sound, sound_group::get_ffi(&sound_group)) }
    }

    pub fn get_sound_group(&self) -> Result<sound_group::SoundGroup, fmod::Result> {
        let mut sound_group = ::std::ptr::mut_null();

        match unsafe { ffi::FMOD_Sound_GetSoundGroup(self.sound, &mut sound_group) } {
            fmod::Ok => Ok(sound_group::from_ptr(sound_group)),
            e => Err(e)
        }
    }

    pub fn get_num_sync_points(&self) -> Result<i32, fmod::Result> {
        let mut num_sync_points = 0i32;

        match unsafe { ffi::FMOD_Sound_GetNumSyncPoints(self.sound, &mut num_sync_points) } {
            fmod::Ok => Ok(num_sync_points),
            e => Err(e)
        }
    }

    pub fn get_sync_point(&self, index: i32) -> Result<FmodSyncPoint, fmod::Result> {
        let mut sync_point = ::std::ptr::mut_null();

        match unsafe { ffi::FMOD_Sound_GetSyncPoint(self.sound, index, &mut sync_point) } {
            fmod::Ok => Ok(FmodSyncPoint::from_ptr(sync_point)),
            e => Err(e)
        }
    }

    pub fn get_sync_point_info(&self, sync_point: FmodSyncPoint, name_len: u32, FmodTimeUnit(offset_type): FmodTimeUnit) -> Result<(String, u32), fmod::Result> {
        let name = String::with_capacity(name_len as uint).into_string();
        let mut offset = 0u32;

        match unsafe { ffi::FMOD_Sound_GetSyncPointInfo(self.sound, sync_point.sync_point, name.with_c_str(|c_name|{c_name as *mut c_char}),
            name_len as i32, &mut offset, offset_type) } {
            fmod::Ok => Ok((name.clone(), offset)),
            e => Err(e)
        }
    }

    pub fn add_sync_point(&self, offset: u32, FmodTimeUnit(offset_type): FmodTimeUnit, name: String) -> Result<FmodSyncPoint, fmod::Result> {
        let mut sync_point = ::std::ptr::mut_null();

        match unsafe { ffi::FMOD_Sound_AddSyncPoint(self.sound, offset, offset_type, name.into_string().with_c_str(|c_name|{c_name}), &mut sync_point) } {
            fmod::Ok => Ok(FmodSyncPoint::from_ptr(sync_point)),
            e => Err(e)
        }
    }

    pub fn delete_sync_point(&self, sync_point: FmodSyncPoint) -> fmod::Result {
        unsafe { ffi::FMOD_Sound_DeleteSyncPoint(self.sound, sync_point.sync_point) }
    }

    pub fn set_mode(&self, FmodMode(mode): FmodMode) -> fmod::Result {
        unsafe { ffi::FMOD_Sound_SetMode(self.sound, mode) }
    }

    pub fn get_mode(&self) -> Result<FmodMode, fmod::Result> {
        let mut mode = 0u32;

        match unsafe { ffi::FMOD_Sound_GetMode(self.sound, &mut mode) } {
            fmod::Ok => Ok(FmodMode(mode)),
            e => Err(e)
        }
    }

    pub fn set_loop_count(&self, loop_count: i32) -> fmod::Result {
        unsafe { ffi::FMOD_Sound_SetLoopCount(self.sound, loop_count) }
    }

    pub fn get_loop_count(&self) -> Result<i32, fmod::Result> {
        let mut loop_count = 0i32;

        match unsafe { ffi::FMOD_Sound_GetLoopCount(self.sound, &mut loop_count) } {
            fmod::Ok => Ok(loop_count),
            e => Err(e)
        }
    }

    pub fn set_loop_points(&self, loop_start: u32, FmodTimeUnit(loop_start_type): FmodTimeUnit, loop_end: u32,
        FmodTimeUnit(loop_end_type): FmodTimeUnit) -> fmod::Result {
        unsafe { ffi::FMOD_Sound_SetLoopPoints(self.sound, loop_start, loop_start_type, loop_end, loop_end_type) }
    }

    pub fn get_loop_points(&self, FmodTimeUnit(loop_start_type): FmodTimeUnit, FmodTimeUnit(loop_end_type): FmodTimeUnit) -> Result<(u32, u32), fmod::Result> {
        let mut loop_start = 0u32;
        let mut loop_end = 0u32;

        match unsafe { ffi::FMOD_Sound_GetLoopPoints(self.sound, &mut loop_start, loop_start_type, &mut loop_end, loop_end_type) } {
            fmod::Ok => Ok((loop_start, loop_end)),
            e => Err(e)
        }
    }

    pub fn get_num_channels(&self) -> Result<i32, fmod::Result> {
        let mut num_channels = 0i32;

        match unsafe { ffi::FMOD_Sound_GetMusicNumChannels(self.sound, &mut num_channels) } {
            fmod::Ok => Ok(num_channels),
            e => Err(e)
        }
    }

    // TODO: see how to replace i32 channel by Channel struct
    pub fn set_music_channel_volume(&self, channel: i32, volume: f32) -> fmod::Result {
        unsafe { ffi::FMOD_Sound_SetMusicChannelVolume(self.sound, channel, volume) }
    }

    // TODO: see how to replace i32 channel by Channel struct
    pub fn get_music_channel_volume(&self, channel: i32) -> Result<f32, fmod::Result> {
        let mut volume = 0f32;

        match unsafe { ffi::FMOD_Sound_GetMusicChannelVolume(self.sound, channel, &mut volume) } {
            fmod::Ok => Ok(volume),
            e => Err(e)
        }
    }

    pub fn set_music_speed(&self, speed: f32) -> fmod::Result {
        unsafe { ffi::FMOD_Sound_SetMusicSpeed(self.sound, speed) }
    }

    pub fn get_music_speed(&self) -> Result<f32, fmod::Result> {
        let mut speed = 0f32;

        match unsafe { ffi::FMOD_Sound_GetMusicSpeed(self.sound, &mut speed) } {
            fmod::Ok => Ok(speed),
            e => Err(e)
        }
    }

    pub fn set_sub_sound_sentence(&self, sub_sounds: &mut Vec<i32>) -> fmod::Result {
        unsafe { ffi::FMOD_Sound_SetSubSoundSentence(self.sound, sub_sounds.as_mut_ptr(), sub_sounds.len() as c_int) }
    }

    pub fn seek_data(&self, pcm: u32) -> fmod::Result {
        unsafe { ffi::FMOD_Sound_SeekData(self.sound, pcm) }
    }

    pub fn get_memory_info(&self, FmodMemoryBits(memory_bits): FmodMemoryBits,
        FmodEventMemoryBits(event_memory_bits): FmodEventMemoryBits) -> Result<(u32, FmodMemoryUsageDetails), fmod::Result> {
        let mut details = fmod_sys::get_memory_usage_details_ffi(Default::default());
        let mut memory_used = 0u32;

        match unsafe { ffi::FMOD_Sound_GetMemoryInfo(self.sound, memory_bits, event_memory_bits, &mut memory_used, &mut details) } {
            fmod::Ok => Ok((memory_used, fmod_sys::from_memory_usage_details_ptr(details))),
            e => Err(e)
        }
    }

    pub fn lock(&self, offset: u32, length: u32) -> Result<(Vec<u8>, Vec<u8>), fmod::Result> {
        let mut len1 = 0u32;
        let mut len2 = 0u32;
        let mut ptr1 =::std::ptr::mut_null();
        let mut ptr2 =::std::ptr::mut_null();

        match unsafe { ffi::FMOD_Sound_Lock(self.sound, offset, length, &mut ptr1, &mut ptr2, &mut len1, &mut len2) } {
            fmod::Ok => {
                let mut v_ptr1 = Vec::new();
                let mut v_ptr2 = Vec::new();

                unsafe { slice::raw::buf_as_slice(ptr1 as *const u8, len1 as uint, |b| {
                   v_ptr1 = Vec::from_slice(b).clone();
                }); }
                unsafe { slice::raw::buf_as_slice(ptr2 as *const u8, len2 as uint, |b| {
                   v_ptr2 = Vec::from_slice(b).clone();
                }); }
                Ok((v_ptr1, v_ptr2))
            }
            e => Err(e)
        }
    }

    pub fn unlock(&self, v_ptr1: Vec<u8>, v_ptr2: Vec<u8>) -> fmod::Result {
        unsafe { ffi::FMOD_Sound_Unlock(self.sound, v_ptr1.as_ptr() as *mut c_void, v_ptr2.as_ptr() as *mut c_void, v_ptr1.len() as c_uint,
            v_ptr2.len() as c_uint) }
    }

    pub fn set_user_data<T>(&mut self, user_data: &mut T) -> fmod::Result {
        let mut data : *mut c_void = ::std::ptr::mut_null();

        unsafe {
            match ffi::FMOD_Sound_GetUserData(self.sound, &mut data) {
                fmod::Ok => {
                    if data.is_null() {
                        self.user_data.user_data = ::std::ptr::mut_null();

                        ffi::FMOD_Sound_SetUserData(self.sound, transmute(&mut self.user_data))
                    } else {
                        let tmp : &mut ffi::SoundData = transmute::<*mut c_void, &mut ffi::SoundData>(data);

                        tmp.user_data = transmute::<&mut T, *mut c_void>(user_data);
                        ffi::FMOD_Sound_SetUserData(self.sound, transmute(tmp))
                    }
                }
                _ => {
                    self.user_data.user_data = transmute::<&mut T, *mut c_void>(user_data);

                    ffi::FMOD_Sound_SetUserData(self.sound, transmute(&mut self.user_data))
                }
            }
        }
    }

    pub fn get_user_data<'r, T>(&'r self) -> Result<&'r mut T, fmod::Result> {
        unsafe {
            let mut user_data : *mut c_void = ::std::ptr::mut_null();

            match ffi::FMOD_Sound_GetUserData(self.sound, &mut user_data) {
                fmod::Ok => {
                    if user_data.is_not_null() {
                        let tmp : &mut ffi::SoundData = transmute::<*mut c_void, &mut ffi::SoundData>(user_data);
                        let tmp2 : &mut T = transmute::<*mut c_void, &mut T>(tmp.user_data);
                        Ok(tmp2)
                    } else {
                        Err(fmod::Ok)
                    }
                },
                e => Err(e)
            }
        }
    }

    pub fn save_to_wav(&self, file_name: &String) -> Result<bool, String> {
        unsafe {
            let mut channels = 0i32;
            let mut bits = 0i32;
            let mut rate = 0f32;
            let len_bytes = match self.get_length(FMOD_TIMEUNIT_PCMBYTES) {
                Ok(l) => l,
                Err(e) => return Err(format!("{}", e))
            };
            let mut len1 = 0u32;
            let mut len2 = 0u32;
            let mut ptr1: *mut c_void =::std::ptr::mut_null();
            let mut ptr2: *mut c_void =::std::ptr::mut_null();

            match ffi::FMOD_Sound_GetFormat(self.sound, ::std::ptr::mut_null(), ::std::ptr::mut_null(), &mut channels, &mut bits) {
                fmod::Ok => match ffi::FMOD_Sound_GetDefaults(self.sound, &mut rate, ::std::ptr::mut_null(), ::std::ptr::mut_null(), ::std::ptr::mut_null()) {
                    fmod::Ok => {}
                    e => return Err(format!("{}", e))
                },
                e => return Err(format!("{}", e))
            };
            let fmt_chunk = FmtChunk {
                chunk: RiffChunk {
                    id: ['f' as i8, 'm' as i8, 't' as i8, ' ' as i8],
                    size: mem::size_of::<FmtChunk>() as i32 - mem::size_of::<RiffChunk>() as i32
                },
                w_format_tag: 1,
                n_channels: channels as u16,
                n_samples_per_sec: rate as u32,
                n_avg_bytes_per_sec: rate as u32 * channels as u32 * bits as u32 / 8u32,
                n_block_align: 1u16 * channels as u16 * bits as u16 / 8u16,
                w_bits_per_sample: bits as u16
            };
            let data_chunk = DataChunk {
                chunk: RiffChunk {
                    id: ['d' as i8, 'a' as i8, 't' as i8, 'a' as i8],
                    size: len_bytes as i32
                }
            };
            let wav_header = WavHeader {
                chunk: RiffChunk {
                    id: ['R' as i8, 'I' as i8, 'F' as i8, 'F' as i8],
                    size: mem::size_of::<FmtChunk>() as i32 + mem::size_of::<RiffChunk>() as i32 + len_bytes as i32
                },
                riff_type: ['W' as i8, 'A' as i8, 'V' as i8, 'E' as i8]
            };

            let file = match File::create(&Path::new(file_name.as_slice())) {
                Ok(f) => f,
                Err(e) => return Err(format!("{}", e))
            };
            let mut buf: BufferedWriter<File> = BufferedWriter::new(file);

            /* wav header */
            for it in range(0u, 4u) {
                buf.write_i8(wav_header.chunk.id[it]).unwrap();
            }
            buf.write_le_i32(wav_header.chunk.size).unwrap();
            for it in range(0u, 4u) {
                buf.write_i8(wav_header.riff_type[it]).unwrap();
            }

            /* wav chunk */
            for it in range(0u, 4u) {
                buf.write_i8(fmt_chunk.chunk.id[it]).unwrap();
            }
            buf.write_le_i32(fmt_chunk.chunk.size).unwrap();
            buf.write_le_u16(fmt_chunk.w_format_tag).unwrap();
            buf.write_le_u16(fmt_chunk.n_channels).unwrap();
            buf.write_le_u32(fmt_chunk.n_samples_per_sec).unwrap();
            buf.write_le_u32(fmt_chunk.n_avg_bytes_per_sec).unwrap();
            buf.write_le_u16(fmt_chunk.n_block_align).unwrap();
            buf.write_le_u16(fmt_chunk.w_bits_per_sample).unwrap();

            /* wav data chunk */
            for it in range(0u, 4u) {
                buf.write_i8(data_chunk.chunk.id[it]).unwrap();
            }
            buf.write_le_i32(data_chunk.chunk.size).unwrap();

            ffi::FMOD_Sound_Lock(self.sound, 0, len_bytes, &mut ptr1, &mut ptr2, &mut len1, &mut len2);

            slice::raw::buf_as_slice(ptr1 as *const u8, len_bytes as uint, |b| {
               buf.write(b).unwrap();
            });

            ffi::FMOD_Sound_Unlock(self.sound, ptr1, ptr2, len1, len2);
        }
        Ok(true)
    }
}