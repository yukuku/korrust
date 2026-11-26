import erasedallocation.mixedAllocator
fun work(value: Any): Array<Class<*>?> = mixedAllocator<Class<*>>(2, value)
