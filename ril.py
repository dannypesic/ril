def rilfn(fn):
    import sys
    frame = sys._getframe(1)
    frame.f_globals['__ril_main__'] = fn
    return fn
