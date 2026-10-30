package annotation17shadow

import kotlin.annotation.Retention as RealRetention
import kotlin.annotation.Target as RealTarget

annotation class Retention(val value: Int)
annotation class Target(val value: Int)

@Retention(7)
@Target(9)
class UserMarked

@RealRetention(AnnotationRetention.RUNTIME)
@RealTarget(AnnotationTarget.CLASS)
annotation class AliasedMark(val mode: annotation17.Mode = annotation17.Mode.SECOND)

@AliasedMark
class AliasedUse
