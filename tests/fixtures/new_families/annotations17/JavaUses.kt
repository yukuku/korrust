package annotation17

import annotation17java.JavaRich
import annotation17java.JavaNested
import java.lang.annotation.RetentionPolicy

@JavaRich
class JavaDefaultUse

@JavaRich("single", number = 9L, nested = JavaNested(text = "explicit", amount = 43L))
class JavaSingleUse

@JavaRich(value = ["left", "right"], type = Token::class, types = [Long::class, Token::class], mode = RetentionPolicy.CLASS, children = [JavaNested(text = "array", amount = 47L)])
class JavaExplicitUse

@JavaRich(*arrayOf())
class JavaEmptyUse
