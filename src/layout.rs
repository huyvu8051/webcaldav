use crate::storage;
use leptos::prelude::*;
use leptos_router::hooks::use_navigate;

#[component]
pub fn TopBar(username: String) -> impl IntoView {
    let navigate = use_navigate();
    let disconnect = move |_| {
        storage::clear();
        navigate("/", Default::default());
    };
    view! {
        <header class="sticky top-0 left-0 right-0 w-full z-50 bg-[#050508]/90 backdrop-blur-xl border-b border-[#2BE8C9]/20">
            <div class="h-16 max-w-[1280px] mx-auto px-4 sm:px-6 flex items-center justify-between">
                <div class="flex items-center gap-3">
                    <div class="w-9 h-9 rounded bg-[#131722] border border-[#2BE8C9]/40 flex items-center justify-center text-[#2BE8C9] shadow-[0_0_12px_rgba(43,232,201,0.3)]">
                        <span class="material-symbols-outlined text-[20px]">"terminal"</span>
                    </div>
                    <div class="flex flex-col">
                        <div class="flex items-center gap-2">
                            <span class="font-brand-display font-bold text-base tracking-wider text-[#e6edf3] uppercase">
                                "WEBCALDAV "<span class="text-[#2BE8C9]">"//"</span>" GRID"
                            </span>
                            <span class="text-[#2BE8C9] font-mono text-[10px] tracking-widest font-semibold border border-[#2BE8C9]/40 px-1.5 py-0.5 rounded bg-[#2BE8C9]/10">
                                "v1.0"
                            </span>
                        </div>
                        <span class="font-mono text-[10px] text-[#8B949E] hidden sm:inline tracking-tight">"RFC-4791 CALDAV CLIENT"</span>
                    </div>
                </div>
                <div class="flex items-center gap-3">
                    <div class="hidden sm:inline-flex items-center gap-2 px-3 py-1 rounded bg-[#0D1117] border border-[#2BE8C9]/20 font-mono text-[11px] text-[#8B949E]">
                        <span class="w-2 h-2 rounded-full bg-[#2BE8C9] animate-pulse"></span>
                        <span class="text-[#2BE8C9] font-semibold">"NODE: ONLINE"</span>
                        <span class="text-white/20">"|"</span>
                        <span>{username}</span>
                    </div>
                    <button
                        class="px-2 py-1 rounded border border-[#2BE8C9]/30 hover:border-[#2BE8C9] text-[#8B949E] hover:text-[#2BE8C9] font-mono text-[11px] transition-colors"
                        on:click=disconnect
                    >
                        "DISCONNECT"
                    </button>
                </div>
            </div>
        </header>
    }
}

#[component]
pub fn Footer() -> impl IntoView {
    view! {
        <footer class="w-full bg-[#050508] border-t border-[#2BE8C9]/20 py-6 text-xs text-[#8B949E] font-mono relative z-10">
            <div class="max-w-[1280px] mx-auto px-4 sm:px-6 flex flex-col md:flex-row items-center justify-between gap-4 text-center sm:text-left">
                <div class="flex items-center gap-2">
                    <span class="inline-block w-2 h-2 rounded-full bg-[#2BE8C9] animate-pulse"></span>
                    <span>"• RFC-4791 CALDAV ENGINE • TLS ENCRYPTED"</span>
                </div>
                <div class="flex items-center gap-4 text-[11px]">
                    <span class="text-[#2BE8C9] font-semibold">"OPENDIY // WEBCALDAV RUNTIME"</span>
                </div>
            </div>
        </footer>
    }
}

pub fn percent_encode(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for byte in input.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char)
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

pub fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(byte) = u8::from_str_radix(&input[i + 1..i + 3], 16) {
                out.push(byte);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}
