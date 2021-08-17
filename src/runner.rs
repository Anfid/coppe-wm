use wasmer::{imports, Function, ImportObject, Instance, Module, NativeFunc, Store};

struct Plugin {
    instance: Instance,
    code: Vec<u8>,
}

pub struct Runner {
    store: Store,
    plugins: Vec<Plugin>,
    events: Vec<u8>,
}

fn import_objects(store: &Store) -> ImportObject {
    let move_window = Function::new_native(store, move_window);
    imports! {
        "env" => {
            "move_window" => move_window,
        }
    }
}

impl Runner {
    pub fn init() -> Result<Runner, ()> {
        use std::fs::File;
        use std::io::Read;

        let store = Store::default();

        let plugin_dir = std::env::var("XDG_CONFIG_HOME")
            .map(|path| {
                let mut path = std::path::PathBuf::from(path);
                path.push("waswm");
                path
            })
            .or_else(|_| {
                std::env::var("HOME").map(|path| {
                    let mut path = std::path::PathBuf::from(path);
                    path.push(".config");
                    path.push("waswm");
                    path
                })
            })
            .unwrap();

        let imports = import_objects(&store);

        let mut plugins = Vec::new();
        for plugin_dir_entry in std::fs::read_dir(plugin_dir).unwrap() {
            let path = plugin_dir_entry.unwrap().path();
            let mut file = File::open(path).unwrap();

            let mut code = Vec::new();
            file.read_to_end(&mut code).unwrap();

            let module = Module::new(&store, &code).unwrap();
            let instance = Instance::new(&module, &imports).unwrap();

            plugins.push(Plugin { instance, code })
        }

        Ok(Runner {
            store,
            plugins,
            events: Vec::new(),
        })
    }

    pub fn run(&mut self) {
        for plugin in &self.plugins {
            let handle: NativeFunc<(), ()> = plugin
                .instance
                .exports
                .get_native_function("handle")
                .unwrap();
            //let memory = instance.exports.get_memory("memory").unwrap();

            handle.call().unwrap();
        }

        self.finalize();
    }

    fn finalize(&mut self) {
        // SAFETY: only called from single-threaded context
        unsafe {
            self.events.extend(&EVENTS);
            EVENTS.clear()
        }
    }
}

static mut EVENTS: Vec<u8> = Vec::new();

fn move_window(id: u32, x: i32, y: i32) {
    // SAFETY: only called from single-threaded context
    unsafe {
        EVENTS.push(id as u8);
    }
    println!("Move window {} to [{}, {}]", id, x, y)
}
