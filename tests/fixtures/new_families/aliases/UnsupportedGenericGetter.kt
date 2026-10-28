package aliasboundaries

import aliasdep.Box

typealias BoxAlias<T> = Box<T>

fun read(value: BoxAlias<String>): String = value.value
fun readWithoutAlias(value: Box<String>): String = value.value
