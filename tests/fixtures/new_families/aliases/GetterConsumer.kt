package aliasgetterconsumer

import aliasboundaries.BoxAlias
import aliasboundaries.read
import aliasboundaries.readWithoutAlias
import aliasdep.Box

fun main() {
    val value = "\u0000\ud83d\ude03"
    val box: BoxAlias<String> = Box(value)
    check(read(box) === value)
    check(readWithoutAlias(box) === value)
    println("OK")
}
