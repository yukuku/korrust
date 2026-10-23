package korrust.probes.io

import fixtures.io.IoTrace

object ByteWorkbench {
    private val octets = ThreadLocal.withInitial { ByteArray(5) }
    private val letters = ThreadLocal.withInitial { CharArray(7) }

    fun instanceCode(): Int = 23
    @JvmStatic
    fun whenValue(value: Int): Int = when (value) { 1 -> 2; else -> 3 }
    @JvmStatic
    fun pick(value: Byte): Int = value.toInt()
    @JvmStatic
    fun pick(value: Int): Int = value + 11
    @JvmStatic
    fun pick(value: ByteArray, index: Int): Byte = value[index]
    @JvmStatic
    fun put(value: ByteArray, index: Int, element: Byte): Byte {
        value[index] = element
        return value[index]
    }
    @JvmStatic
    fun narrow(value: Int): Byte = value.toByte()
    @JvmStatic
    fun fill(value: CharArray, start: Int, element: Char): Int {
        var cursor = start
        value[cursor++] = element
        return cursor
    }
    @JvmStatic
    fun ordered(trace: IoTrace): Int {
        trace.array()[trace.index()] = trace.value()
        return trace.ticks()
    }
    @JvmStatic
    fun orderedChars(trace: IoTrace): Int {
        trace.charArray()[trace.index()] = trace.charValue()
        return trace.ticks()
    }
    @JvmStatic
    fun constructed(trace: IoTrace): String = String(trace.charArray(), trace.index(), trace.length())
    @JvmStatic
    fun fold(value: ByteArray): Int {
        var total = 0
        for (index in 0 until value.size) {
            val code = value[index].toInt() and 255
            if (code == 0) continue
            if (code == 255) return total
            total += code
        }
        return total
    }
    @JvmStatic
    fun classify(value: Byte): Int {
        val code = value.toInt() and 255
        val result = when (code) {
            in 0..31 -> 3
            in 32..126 -> 5
            else -> 9
        }
        return result
    }
    @JvmStatic
    fun assigned(flag: Boolean, left: Int, right: Int): Int {
        var result: Int
        if (flag) { result = left } else { result = right }
        return result
    }
    @JvmStatic
    fun endpoints(trace: IoTrace): Int {
        var sum = 0
        for (index in 0 until trace.forEnd()) { sum += index }
        return sum
    }
    @JvmStatic
    fun ranged(trace: IoTrace): Boolean = trace.subject() in trace.low()..trace.high()
    @JvmStatic
    fun selected(trace: IoTrace): Int = when (trace.subject()) {
        in 0..5 -> 7
        else -> 9
    }
    @JvmStatic
    fun scratchBytes(): ByteArray = octets.get()!!
    @JvmStatic
    fun scratchChars(): CharArray = letters.get()!!
    @JvmStatic
    fun setBytes(value: ByteArray) { octets.set(value) }
    @JvmStatic
    fun chars(value: ByteArray): String {
        val output = CharArray(value.size)
        var pos = 0
        while (pos < value.size) {
            output[pos] = (value[pos].toInt() and 255).toChar()
            pos++
        }
        return String(output, 0, pos)
    }
    @JvmStatic
    fun resized(value: CharArray, length: Int): CharArray = value.copyOf(length)
}
