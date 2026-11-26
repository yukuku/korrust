package consumers

import dep.Factory

fun bad(): Int = Factory.empty<Int>()
