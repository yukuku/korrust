package annotation17cases
import annotation17.*
annotation class Left(val right: Right)
annotation class Right(val left: Left)
