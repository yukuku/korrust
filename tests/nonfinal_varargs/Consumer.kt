package nonfinal.consumer

import nonfinal.library.Collected
import nonfinal.library.Collector
import nonfinal.library.choose
import nonfinal.library.collect
import nonfinal.library.pack
import nonfinal.library.record
import nonfinal.library.tail

fun empty(): Int = pack()
fun filled(): Int = pack(3, record(4), record(5), suffix = record(6)) { it + 7 }
fun reordered(): Int = pack(suffix = record(8), values = intArrayOf(record(4), record(5)), prefix = record(1))
fun spread(): Int = pack(2, *intArrayOf(3, 4), 5, suffix = 6) { it }
fun generic(): String = tail("first", "second", last = "last")
fun genericEmpty(): String = tail(last = "last")
fun member(): Int = Collector().pack(1, 2, suffix = 3)
fun constructor(): Int = Collected(1, 2, suffix = 3).total

fun extension(): Int = 10.collect(1, 2, suffix = 3)

fun overload(): Int = choose(1) { 2 }
