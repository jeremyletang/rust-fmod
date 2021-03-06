/*
* Rust-FMOD - Copyright (c) 2014 Gomez Guillaume.
*
* The Original software, FMOD library, is provided by FIRELIGHT TECHNOLOGIES.
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

#![feature(globs)]
#![allow(non_snake_case_functions)]
#![allow(unused_variable)]

extern crate rfmod;

use rfmod::enums::*;
use rfmod::*;
use rfmod::types::FmodMode;
use std::os;
use std::default::Default;

fn get_key() -> u8 {
    let mut reader = std::io::stdio::stdin();
    print!("> ");

    match reader.read_byte() {
        Ok(nb) => nb,
        Err(_) => 0u8
    }
}

#[allow(unused_variable)]
fn my_DSP_callback(dsp_state: &DspState, inbuffer: &mut Vec<f32>, outbuffer: &mut Vec<f32>, length: u32, inchannels: i32, outchannels: i32) -> fmod::Result {
    for it in range(0u, inbuffer.len() - 1u) {
        *outbuffer.get_mut(it) = *inbuffer.get_mut(it) * 0.2f32;
    }

    fmod::Ok
}

fn main() {
    let args = os::args();
    let tmp = args.tail();

    if tmp.len() < 1 {
        fail!("USAGE: ./dsp_custom [music_file]");
    }
    let fmod = match FmodSys::new() {
        Ok(f) => f,
        Err(e) => {
            fail!("FmodSys.new : {}", e);
        }
    };

    match fmod.init() {
        fmod::Ok => {}
        e => {
            fail!("FmodSys.init failed : {}", e);
        }
    };

    let arg1 = tmp.get(0).unwrap();

    let sound = match fmod.create_sound((*arg1).as_slice(), Some(FmodMode(FMOD_SOFTWARE | FMOD_LOOP_NORMAL)), None) {
        Ok(s) => s,
        Err(err) => {fail!("FmodSys.create_sound failed : {}", err);}
    };

    println!("============================");
    println!("======== Custom DSP ========");
    println!("============================\n");
    println!("Enter 'f' to activate / deactivate user filter");
    println!("Enter 'Esc' to quit");

    let channel = match sound.play() {
        Ok(c) => c,
        Err(e) => {fail!("Sound.play failed : {}", e);}
    };

    let mut description : DspDescription = Default::default();
    description.read = Some(my_DSP_callback);
    description.name = String::from_str("test");

    let dsp = match fmod.create_DSP_with_description(&mut description) {
        Ok(dsp) => dsp,
        Err(e) => {fail!("FmodSys.create_DSP_with_description failed : {}", e);}
    };

    dsp.set_bypass(true);
    let connection = match fmod.add_DSP(&dsp) {
        Ok(c) => c,
        Err(e) => {fail!("FmodSys.add_DSP failed : {}", e);}
    };

    let mut active = false;
    loop {
        match get_key() as char {
            'f' => {
                dsp.set_bypass(active);
                active = !active;
                fmod.update();
            }
            c if c == 27u8 as char => break,
            _ => {}
        }
    }
}