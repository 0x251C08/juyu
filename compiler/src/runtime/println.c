static void juyu_io_println(juyu_str msg) {
    if (fwrite(msg.ptr, 1, msg.len, stdout) != msg.len || fputc('\n', stdout) == EOF) abort();
}
