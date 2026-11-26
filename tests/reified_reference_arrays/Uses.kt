package erasedconsumer
import erasedallocation.Box
import erasedallocation.arbitraryAllocator

fun classes(): Array<Class<*>> = arrayOf<Class<*>>(String::class.java, Int::class.java)
fun emptyClasses(): Array<Class<*>> = emptyArray<Class<*>>()
fun nullableClasses(): Array<Class<String>?> = arbitraryAllocator<Class<String>>(2)
fun boxes(): Array<Box<String>?> = arbitraryAllocator<Box<String>>(2)
fun lists(): Array<List<String>?> = arbitraryAllocator<List<String>>(2)
fun <U: Any> genericClasses(size: Int): Array<Class<U>?> = arbitraryAllocator<Class<U>>(size)

private var rhsCalls: Int = 0
fun resetFallbackCalls() { rhsCalls = 0 }
fun fallbackCalls(): Int = rhsCalls
private fun makeFallback(): Array<Class<*>> { rhsCalls += 1; return emptyArray() }
fun inferredFallback(value: Array<Class<*>>?): Array<Class<*>> = value ?: emptyArray()
fun lazyFallback(value: Array<Class<*>>?): Array<Class<*>> = value ?: makeFallback()
fun nullableFallback(value: String?, other: String?): String? = value ?: other
fun nullLeftFallback(): Array<Class<*>> = null ?: emptyArray()
