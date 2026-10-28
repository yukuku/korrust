package aliasboundaries

import aliasdep.Box

typealias BoxAlias<T> = Box<T>

fun create(): BoxAlias<String> = BoxAlias("OK")
