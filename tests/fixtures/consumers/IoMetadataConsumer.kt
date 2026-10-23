package korrust.consumers

import korrust.probes.io.ByteWorkbench
import korrust.probes.io.LazyScratch
import yuku.alkitab.io.Utf8Decoder

fun main() {
    val bytes = byteArrayOf(65, -1, 0)
    check(ByteWorkbench.pick((-1).toByte()) == -1)
    check(ByteWorkbench.pick(4) == 15)
    check(ByteWorkbench.pick(bytes, 0) == 65.toByte())
    check(ByteWorkbench.put(bytes, 0, (-128).toByte()) == (-128).toByte())
    check(ByteWorkbench.assigned(true, 7, 9) == 7)
    check(LazyScratch.constant() == 17)
    check(Utf8Decoder.toString("İ𐐀".toByteArray()) == "İ𐐀")
    check(Utf8Decoder.toStringLowerCase("İ𐐀".toByteArray()) == "i𐐨")
    println("byte/object-overload/decoder Kotlin metadata consumer OK")
}
