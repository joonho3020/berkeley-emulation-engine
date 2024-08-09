# Opal kelly board flow

## Install Xilinx ISE

- Download the xilinx ise installer:

```bash
wget https://www.xilinx.com/member/forms/download/xef.html?filename=Xilinx_ISE_DS_Lin_14.7_1015_1.tar
```

- unzip the above file and run:

```bash
cd <unzipped dir>
./xsetup
```

```
cd 14.7/ISE_DS/common/lib/lin64
./xlcm -manage
```

- The above step will probably throw and error complaining about missing `libQt_Network`
- Download the `libQt_Network` shared library from [xilinx support](https://support.xilinx.com/s/article/58400?language=en_US)
- Run:

```bash
unzip libQt_Network.zip
mv libQt_Network_* 14.7/ISE_DS/common/lib/lin64/
chmod 755 libQt_Network_*
```

- Also, you may have to ask for a license from xilinx: [Xilinx license support](https://support.xilinx.com/s/question/0D52E00006iHqRLSA0/get-a-free-vivadoise-webpack-license-and-start-using-your-xilinx-software?language=en_US)


## Building bitstreams using ISE

- To launch the ISE gui, run:

```bash
cd <ISE install dir>/14.7/ISE_DS/ISE/bin/lin64
./ise
```

- Now it is just like vivado: you can import source files and constraint files, lauch bitstream builds etc

## The frontpanel constraints file is in another place...
