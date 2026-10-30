package annotation17cases
import annotation17.*
fun computed(): Long = 7L
@Rich(number = computed()) class Bad
