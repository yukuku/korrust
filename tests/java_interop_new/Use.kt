package newinterop.use

import newinterop.Bean
import newinterop.ChildBean
import newinterop.Factory
import newinterop.GenericBean
import newinterop.Bean.choose as selected
import newinterop.Bean.sum
import newinterop.ChildBean.choose as inherited
import newinterop.ChildBean.CONSTANT as inheritedConstant
import newinterop.Bean.counter as count
import newinterop.Facade
import newinterop.Facade.inheritedStatic as inheritedHidden
import newinterop.Facade.inheritedCounter as hiddenCount

fun staticCalls(): String = "${selected(2)}${selected("a")}${inherited(3)}:${sum(1, 2, 3)}:$inheritedConstant:$count"

fun properties(bean: ChildBean): String {
    val before = bean.title
    bean.title = "next"
    bean.isReady = true
    return "$before:${bean.title}:${bean.isReady}:${bean.urlValue}:${bean.exposed}:${bean.readOnly}"
}

fun evaluationOrder(): String {
    Factory.receiver().title = Factory.argument()
    return Factory.receiver().title
}

fun nullable(bean: Bean?): String? = bean?.title

fun generic(bean: GenericBean<String>): String {
    val first: String = bean.value
    bean.value = first + "K"
    return bean.value
}

fun objectMembers(value: Bean): Boolean = value.equals(other = value) && value.toString().isNotEmpty() && value.hashCode() == value.hashCode()

fun hiddenParent(value: Facade): String {
    hiddenCount = hiddenCount + 1
    return inheritedHidden() + ":" + value.inheritedTitle + ":" + hiddenCount
}

fun nullWrite(value: Bean): String? {
    value.title = null
    return value.title
}

fun objectHash(value: Bean): Int = value.hashCode()
fun platformConcat(): String = Bean.nullableText() + "K"
fun concatObject(value: Bean): String = "bean:" + value
fun emptySum(): Int = sum()
