static void juyu_io_println_int(int64_t value) {
    if (printf("%" PRId64 "\n", value) < 0) abort();
}
