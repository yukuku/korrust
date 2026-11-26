import erasedallocation.arbitraryAllocator
fun <U> work(size: Int): Array<U?> = arbitraryAllocator<U>(size)
