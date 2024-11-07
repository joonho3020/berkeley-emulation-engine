// import Mill dependency
import mill._
import mill.define.Sources
import mill.modules.Util
import mill.scalalib.TestModule.ScalaTest
import scalalib._
// support BSP
import mill.bsp._

object emulator extends ScalaModule {
  def millSourcePath = os.pwd
  def scalaVersion = "2.13.12"

  def scalacOptions = Seq(
    "-language:reflectiveCalls",
    "-deprecation",
    "-feature",
    "-Xcheckinit",
    "-Ymacro-annotations",
  )
  override def ivyDeps = Agg(
    ivy"org.chipsalliance::chisel:6.0.0",
    ivy"edu.berkeley.cs::rocketchip-6.0.0:1.6-6.0.0-1b9f43352-SNAPSHOT"
  )
  override def scalacPluginIvyDeps = Agg(
    ivy"org.chipsalliance:::chisel-plugin:6.0.0",
  )

  // https://mill-build.org/mill/0.11.12/Scala_Module_Config.html
  val sonatypeReleases = Seq(
    coursier.maven.MavenRepository("https://oss.sonatype.org/content/repositories/snapshots/"),
    coursier.maven.MavenRepository("https://oss.sonatype.org/content/repositories/releases/"),
  )
  def repositoriesTask = T.task {
    super.repositoriesTask() ++ sonatypeReleases
  }
}
