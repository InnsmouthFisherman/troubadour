use cpal::{Device, traits::{DeviceTrait, HostTrait}};
use std::{fs, path::PathBuf, sync::{Arc, Mutex}};
use std::env;
use rdev::{listen, Event, EventType, Key};
use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink, Source};
use std::io::BufReader;
use std::fs::File;
use anyhow::Result;

// Состояние для отслеживания зажатых клавиш-модификаторов
struct ModifierState {
    alt_pressed: bool,
}

impl ModifierState {
    fn new() -> Self {
        Self {
            alt_pressed: false,
        }
    }
}

// Глобальное состояние приложения
struct AppState {
    sounds: Vec<PathBuf>,
    stream_handle: OutputStreamHandle,
}

impl AppState {
    fn new() -> Result<Self> {
        let (_stream, stream_handle) = OutputStream::try_default()?;
        
        // Сохраняем _stream в статическую переменную, чтобы он не был уничтожен
        // Это небольшой хак, но он работает
        std::mem::forget(_stream);
        
        Ok(Self {
            sounds: Vec::new(),
            stream_handle,
        })
    }
    
    fn play_sound(&self, index: usize) -> Result<()> {
        if index >= self.sounds.len() {
            return Err(anyhow::anyhow!("Индекс вне диапазона"));
        }
        
        let path = self.sounds[index].clone();
        let stream_handle = self.stream_handle.clone();
        
        // Запускаем воспроизведение в отдельном потоке
        std::thread::spawn(move || {
            if let Err(e) = play_file(stream_handle, path) {
                eprintln!("❌ Ошибка воспроизведения: {}", e);
            }
        });
        
        Ok(())
    }
}

// Отдельная функция для воспроизведения файла
fn play_file(stream_handle: OutputStreamHandle, path: PathBuf) -> Result<()> {
    let file = BufReader::new(File::open(path)?);
    let source = Decoder::new(file)?;
    
    let sink = Sink::try_new(&stream_handle)?;
    sink.append(source);
    sink.sleep_until_end();
    
    Ok(())
}

fn main() {
    println!("{}, version {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));

    // Инициализируем состояние приложения
    let state = match AppState::new() {
        Ok(mut s) => {
            // Загружаем звуки
            s.sounds = locate_sound();
            if s.sounds.is_empty() {
                eprintln!("❌ Нет звуковых файлов в папке 'sounds'");
                return;
            }
            println!("✅ Загружено {} звуковых файлов", s.sounds.len());
            Arc::new(s)
        }
        Err(e) => {
            eprintln!("❌ Ошибка инициализации аудио: {}", e);
            return;
        }
    };

    // Находим устройство для виртуального кабеля (опционально)
    match configure_device() {
        Ok(device) => println!("Использую устройство: {}", device.name().unwrap()),
        Err(_) => println!("⚠️ Виртуальный кабель не найден, звук будет идти в динамики"),
    }

    // Настраиваем горячие клавиши
    configure_hotkeys(Arc::clone(&state));
    
    println!("\n🎹 Soundpad запущен! Используйте Alt+[1..{}] для воспроизведения звуков", 
        state.sounds.len().min(9));
    println!("Нажмите Ctrl+C для выхода");
    
    // Держим программу запущенной
    loop {
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}

fn locate_sound() -> Vec<PathBuf> {
    let mut sounds = Vec::new();
    let mut path_to_samples = env::current_dir().expect("error getting current dir");
    path_to_samples.push("sounds");

    if path_to_samples.exists() {
        println!("📁 Поиск в папке: {:?}", path_to_samples);
        
        for entry in fs::read_dir(path_to_samples).expect("unable to list") {
            let path = entry.unwrap().path();
            
            if let Some(ext) = path.extension() {
                let ext_str = ext.to_string_lossy().to_lowercase();
                if ext_str == "mp3" || ext_str == "wav" || ext_str == "ogg" || ext_str == "flac" {
                    println!("  🎵 Найден: {:?}", path.file_name().unwrap());
                    sounds.push(path);
                }
            }
        }
    } else {
        eprintln!("❌ Папка 'sounds' не найдена");
        fs::create_dir_all("sounds").expect("не удалось создать папку sounds");
        println!("✅ Создана папка 'sounds'. Положите в неё MP3 файлы и перезапустите программу");
    }

    sounds.sort();
    sounds
}

fn configure_hotkeys(state: Arc<AppState>) {
    println!("\n🎹 Назначенные горячие клавиши (Alt+):");
    for (i, sound) in state.sounds.iter().enumerate() {
        if i < 9 {
            let file_name = sound.file_name().unwrap().to_string_lossy();
            println!("  Alt+{}: {}", i + 1, file_name);
        }
    }
    if state.sounds.len() > 9 {
        println!("  ... и ещё {} звуков (не привязаны)", state.sounds.len() - 9);
    }

    // Запускаем прослушивание клавиш в отдельном потоке
    std::thread::spawn(move || {
        let modifier_state = Arc::new(Mutex::new(ModifierState::new()));
        
        if let Err(error) = listen(move |event| {
            handle_key_event(event, &modifier_state, &state);
        }) {
            eprintln!("Ошибка прослушивания клавиш: {:?}", error);
        }
    });
}

fn handle_key_event(
    event: Event, 
    modifier_state: &Arc<Mutex<ModifierState>>,
    state: &Arc<AppState>,
) {
    match event.event_type {
        EventType::KeyPress(key) => {
            match key {
                Key::Alt => {
                    modifier_state.lock().unwrap().alt_pressed = true;
                }
                Key::Num1 | Key::Num2 | Key::Num3 | Key::Num4 | Key::Num5 |
                Key::Num6 | Key::Num7 | Key::Num8 | Key::Num9 => {
                    if modifier_state.lock().unwrap().alt_pressed {
                        let index = match key {
                            Key::Num1 => 0,
                            Key::Num2 => 1,
                            Key::Num3 => 2,
                            Key::Num4 => 3,
                            Key::Num5 => 4,
                            Key::Num6 => 5,
                            Key::Num7 => 6,
                            Key::Num8 => 7,
                            Key::Num9 => 8,
                            _ => unreachable!(),
                        };
                        
                        if index < state.sounds.len() {
                            let file_name = state.sounds[index].file_name().unwrap().to_string_lossy();
                            println!("▶️ Alt+{}: {}", index + 1, file_name);
                            
                            // Воспроизводим через состояние
                            if let Err(e) = state.play_sound(index) {
                                eprintln!("❌ Ошибка воспроизведения: {}", e);
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        EventType::KeyRelease(key) => {
            if key == Key::Alt {
                modifier_state.lock().unwrap().alt_pressed = false;
            }
        }
        _ => {}
    }
}

fn configure_device() -> Result<Device, anyhow::Error> {
    let host = cpal::default_host();

    let device = host.output_devices()?
        .find(|d| {
            if let Ok(name) = d.name() {
                name.contains("CABLE Input") || name.contains("VB-Audio")
            } else {
                false
            }
        });
    
    match device {
        Some(d) => Ok(d),
        None => Err(anyhow::anyhow!("CABLE Input не найден")),
    }
}