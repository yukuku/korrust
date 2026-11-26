package erasedconsumer
import erasedallocation.Box
import erasedallocation.arbitraryAllocator

fun classes(): Array<Class<*>> = arrayOf<Class<*>>(String::class.java, Int::class.java)
fun emptyClasses(): Array<Class<*>> = emptyArray<Class<*>>()
fun nullableClasses(): Array<Class<String>?> = arbitraryAllocator<Class<String>>(2)
fun boxes(): Array<Box<String>?> = arbitraryAllocator<Box<String>>(2)
fun lists(): Array<List<String>?> = arbitraryAllocator<List<String>>(2)
fun <U: Any> genericClasses(size: Int): Array<Class<U>?> = arbitraryAllocator<Class<U>>(size)
