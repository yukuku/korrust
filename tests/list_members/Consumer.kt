package slices

import fixture.subList

fun read(values: List<String>, low: Int, high: Int): List<String> = values.subList(low, high)
fun writable(values: MutableList<String>): MutableList<String> = values.subList(1, 3)
fun where(values: List<String>, key: String): Int = values.indexOf(key)
fun last(values: List<String>, key: String): Int = values.lastIndexOf(key)
fun element(values: List<String>): String = values.subList(0, 1)[0]
fun alternate(values: List<String>): String = values.subList("[", "]")
fun render(values: List<String>): String = values.toString()
