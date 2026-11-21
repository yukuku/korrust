package aliasprobe

import classifierprobe.Catalog as Renamed

fun alias(value: Renamed.Entry<String>): String = value.key
fun fullyQualified(value: classifierprobe.Catalog.Entry<String>): String = value.key
