package consumers

import dep.Arrays

fun sizes(values: Array<String?>, nested: Array<Array<String>>, ints: Array<IntArray>): Int =
    Arrays.objects(values) + Arrays.strings(values) + Arrays.nested(nested) + Arrays.objects(ints)

fun write(values: Array<String>) { Arrays.write(values) }
