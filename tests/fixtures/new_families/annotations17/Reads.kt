package annotation17reads

import annotation17.Rich
import annotation17java.JavaRich
import kotlin.reflect.KClass

fun scalar(value: Rich): KClass<*> = value.type
fun array(value: Rich): Array<KClass<*>> = value.types
fun javaScalar(value: JavaRich): KClass<*> = value.type
fun javaArray(value: JavaRich): Array<KClass<*>> = value.types
fun nested(value: Rich): Long = value.nested.amount
fun elements(value: Rich): Int = value.numbers.size + value.children.size
fun supplied(supply: () -> Rich): KClass<*> = supply().type
fun suppliedArray(supply: () -> Rich): Array<KClass<*>> = supply().types
