import rerun as rr

rr.init("my_test_run")
rr.log("world", rr.TextDocument("Hello, rerun!"))
rr.save("test.rrd")
