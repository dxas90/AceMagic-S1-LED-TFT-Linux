#!/usr/bin/env python
""" LCD Display module for Acemgic S1 family of mini PCs.
This program is free software: you can redistribute it under
the terms of the GNU General Public License as published by the Free Software
Foundation, either version 3 of the License.
This program is distributed in the hope that it will be useful, but WITHOUT
ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS
FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details.
You should have received a copy of the GNU General Public License along with
this program. If not, see <http://www.gnu.org/licenses/>.
"""

__author__ = "Venture Misquitta"
__contact__ = "venturecoder@gmail.com"
__copyright__ = "Copyright Techsperiments Inc."
__credits__ = ["https://github.com/tjaworski/AceMagic-S1-LED-TFT-Linux/commits?author=tjaworski"]
__date__ = "9th Nov 2024"
__email__ =  "venturecoder@gmail.com"
__license__ = "GPLv3"
__maintainer__ = "venture"
__status__ = "Production"
__version__ = "1.0.0"

import random
import usb.core
import usb.backend.libusb1
from time import sleep
from os import read
import time
from datetime import datetime
from PIL import Image, ImageColor, ImageDraw, ImageDraw2, ImageFont, ImageOps, ImageChops
import sched
import glob
from pprint import pprint
import math
import psutil

VENDOR_ID  = 0x04d9  # OnTrak Control Systems Inc. vendor ID
PRODUCT_ID = 0xfd01  # ADU100 Device product name - change this to match your product

# The `LCDObject` class defines an object with default attributes and a method for drawing on a
# background image and a text image.
class LCDObject:
    type="base"
    def __init__(self):
        """
        The function initializes an object with x and y attributes set to 50.
        """
        print(f"Creating object...")
        self.x=50
        self.y=50
    
    def draw(self,bgImage:Image, txtIimage:Image):
        """
        The `draw` function takes two Image objects as input and prints ":Default Render:".
        
        :param bgImage: The `bgImage` parameter is an image that serves as the background for the rendering
        process. It is typically used as the base image onto which other elements, such as text or graphics,
        are added
        :type bgImage: Image
        :param txtIimage: The `txtIimage` parameter is likely referring to an image that contains text or
        some kind of textual content that needs to be rendered onto the `bgImage` background image. This
        parameter is used in the `draw` method to combine the text image with the background image for
        rendering
        :type txtIimage: Image
        """
        print(":Default Render:")
        pass

# This Python class defines an LCDText object with properties such as text, size, font, and methods
# for drawing text on an image.
class LCDText(LCDObject):
    type="text"
    def __init__(self):
        super().__init__()
        self.text="DEF"
        self.size=12
        self.font=ImageFont.load_default(self.size)

    def __init__(self,x,y,text="TEXT", textColor:ImageColor=(255,0,255), fontName="default", fontSize=14):
        """
        This Python function initializes an object with specified coordinates, text, text color, font name,
        and font size.
        
        :param x: The `x` parameter in the `__init__` method is used to specify the x-coordinate of the text
        position on the image
        :param y: The `y` parameter in the `__init__` method of the class is used to specify the
        y-coordinate of the text that will be displayed. It determines the vertical position of the text on
        the screen or image where it will be rendered
        :param text: The `text` parameter is a string that represents the text content that will be
        displayed at the specified coordinates (x, y) on an image. It is set to a default value of "TEXT" if
        no value is provided when initializing an instance of the class, defaults to TEXT (optional)
        :param textColor: The `textColor` parameter in the `__init__` method is a tuple representing the RGB
        color values for the text color. The default value is set to magenta (255, 0, 255)
        :type textColor: ImageColor
        :param fontName: The `fontName` parameter in the `__init__` method is used to specify the name of
        the font to be used for rendering text. This parameter allows the user to choose a specific font for
        displaying the text. If the specified font is not found, the code attempts to load a default font,
        defaults to default (optional)
        :param fontSize: The `fontSize` parameter in the `__init__` method is used to specify the size of
        the font that will be used to render the text. It determines how big or small the text will appear
        on the screen or image, defaults to 14 (optional)
        """
        super().__init__()
        self.x,self.y=x,y
        self.text=text
        self.size=fontSize       
        try:
            self.font=ImageFont.truetype(fontName,self.size)
        except:
            self.font=ImageFont.load_default(self.size)
        self.font=ImageFont.load_default(self.size)
        self.color=textColor

    def draw(self,bgImage:Image,txtImage:Image):
        """
        The `draw` function takes a background image and a text image, renders the text on the text image,
        and returns the text image with the bounding box of the rendered text.
        
        :param bgImage: The `bgImage` parameter is an image that serves as the background on which the text
        will be rendered. It is of type `Image`
        :type bgImage: Image
        :param txtImage: The `txtImage` parameter in the `draw` method is an image on which text will be
        rendered. This image will be modified by drawing text on it using the specified font, color, and
        position
        :type txtImage: Image
        :return: The `draw` method returns a tuple containing the `txtImage` and the `bounds` of the text
        that was drawn on the image.
        """
        print(f":Text Render: {txtImage}")        
        draw = ImageDraw.Draw(txtImage)
        # font = ImageFont.load_default(28)
        bounds = draw.textbbox((self.x,self.y) , text=self.text, font=self.font)
        draw.text((self.x,self.y) , text=self.text, font=self.font, fill=self.color)
        return(txtImage,bounds)


# The `LCDTime` class extends `LCDText` to display the current time in HH:MM:SS format with a bounding
# box and timestamp on an image.
class LCDTime(LCDText):
    type="time"
    def __init__(self,x,y,textColor:ImageColor=(0,255,255), fontName="default", fontSize=34):
        super().__init__(x,y,"TIME",textColor,fontName, fontSize)

    def draw(self,bgImage:Image,txtImage:Image):
        """
        The `draw` function takes an image as a background and another image with text, adds a timestamp to
        the text image, draws a rectangle around the timestamp, and returns the modified text image with its
        bounding box.
        
        :param bgImage: The `bgImage` parameter is of type `Image` and represents the background image on
        which you want to draw the text. It is used as a reference for positioning the text
        :type bgImage: Image
        :param txtImage: The `txtImage` parameter in the `draw` method is an image on which text will be
        drawn. This image will be modified by adding the current time in the format HH:MM:SS. The method
        will draw the text on this image using the specified font, color, and position
        :type txtImage: Image
        :return: The `draw` method returns the `txtImage` with the text and formatting applied, as well as
        the bounding box of the text drawn on the image.
        """
        
        draw = ImageDraw.Draw(txtImage)
        self.text = f"{time.strftime('%H:%M:%S')}"
        print(f":Time Render: {self.text}")                
        bounds = draw.textbbox((self.x,self.y) , text=self.text, font=self.font)
        draw.rectangle(bounds,fill=(0,0,0,125))
        draw.text((self.x,self.y) , text=self.text, font=self.font, fill=self.color, stroke_fill=(255,0,0))
        return(txtImage,bounds)

# The `LCDDate` class is a subclass of `LCDText` that displays the current month and year in a
# specified format on a background image.
class LCDDate(LCDText):
    type="date"
    def __init__(self,x,y,textColor:ImageColor=(0,255,255), fontName="default", fontSize=34):
        super().__init__(x,y,"TIME",textColor,fontName, fontSize)

    def draw(self,bgImage:Image,txtImage:Image):        
        """
        This Python function draws a text string representing the current month and year on an image with
        specified background and text images.
        
        :param bgImage: The `bgImage` parameter is an image that serves as the background on which the text
        will be drawn. It is of type `Image`
        :type bgImage: Image
        :param txtImage: The `txtImage` parameter in the `draw` method is an Image object where the text
        will be drawn. This image will be modified by drawing the text and a rectangle around it based on
        the provided parameters and attributes
        :type txtImage: Image
        :return: The `draw` method returns the `txtImage` with the text rendered on it along with the
        bounding box of the text.
        """
        draw = ImageDraw.Draw(txtImage)
        self.text = f"{datetime.now().strftime('%B %Y')}"
        print(f":Date Render: {self.text}")                
        bounds = draw.textbbox((self.x,self.y) , text=self.text, font=self.font)
        draw.rectangle(bounds,fill=(0,0,0,125))
        draw.text((self.x,self.y) , text=self.text, font=self.font, fill=self.color, stroke_fill=(255,0,0))
        return(txtImage,bounds)

# This class represents a CPU utilization display on an LCD screen with the ability to render and
# update the CPU percentage.
class LCDCPUutil(LCDText):
    type="date"
    def __init__(self,x,y,textColor:ImageColor=(0,255,255), fontName="default", fontSize=34):
        super().__init__(x,y,"TIME",textColor,fontName, fontSize)

    def draw(self,bgImage:Image,txtImage:Image):        
        """
        The `draw` function takes in a background image and a text image, draws CPU utilization text on the
        text image, and returns the modified text image with bounding box coordinates.
        
        :param bgImage: The `bgImage` parameter in the `draw` method is of type `Image` and represents the
        background image on which the text will be drawn
        :type bgImage: Image
        :param txtImage: The `txtImage` parameter in the `draw` method is an Image object where the text
        will be drawn. This image will be modified by drawing the CPU utilization text on it
        :type txtImage: Image
        :return: The `draw` method returns the `txtImage` with the text rendered on it along with the
        bounding box of the text.
        """
        draw = ImageDraw.Draw(txtImage)
        self.text = f"{psutil.cpu_percent():^4}%"
        print(f":CPU Util Render: {self.text}")                
        bounds = draw.textbbox((self.x,self.y) , text=self.text, font=self.font)
        draw.rectangle(bounds,fill=(0,0,0,125))
        draw.text((self.x,self.y) , text=self.text, font=self.font, fill=self.color, stroke_fill=(255,0,0))
        return(txtImage,bounds)

# This Python class, LCDCPUfreq, is a subclass of LCDText that displays the current CPU frequency on
# an LCD screen with specified text color, font, and size.
class LCDCPUfreq(LCDText):
    type="date"
    def __init__(self,x,y,textColor:ImageColor=(0,255,255), fontName="default", fontSize=34):
        super().__init__(x,y,"TIME",textColor,fontName, fontSize)

    def draw(self,bgImage:Image,txtImage:Image):        
        """
        This function draws CPU utilization information on an image with a background image.
        
        :param bgImage: The `bgImage` parameter is an image that serves as the background on which the text
        will be drawn. It is of type `Image`
        :type bgImage: Image
        :param txtImage: The `txtImage` parameter in the `draw` method is an image on which text will be
        drawn. This image will be modified by adding text related to CPU utilization
        :type txtImage: Image
        :return: The `draw` method returns the `txtImage` with the CPU utilization text drawn on it along
        with the bounding box of the text.
        """
        draw = ImageDraw.Draw(txtImage)
        self.text = f"{round(psutil.cpu_freq().current,0)}"
        print(f":CPU Util Render: {self.text}")                
        bounds = draw.textbbox((self.x,self.y) , text=self.text, font=self.font)
        draw.rectangle(bounds,fill=(0,0,0,125))
        draw.text((self.x,self.y) , text=self.text, font=self.font, fill=self.color, stroke_fill=(255,0,0))
        return(txtImage,bounds)


class S1TFT:
    """
    Raises:
        ValueError: [description]

    Returns:
        [type]: [description]
    """
    device = None
    endpoint = None
    scheduler = None
    font_name = "DEFAULT"
    font = ImageFont.load_default(12)
    imageFileList = []
    timeCounter = 0
    
    # True is vertical and false is horizontal
    def __init__(self, d_width, d_height, isVertical:bool = False, font_name="DEFAULT"):
        """ Args:
            width ([type]): [description]
            height ([type]): [description]
            d_width ([type]): [description]
            d_height ([type]): [description]
            orientation (bool): [description]
            font_name (str, optional): [description]. Defaults to "DEFAULT".
        """ 
        self.device = usb.core.find(idVendor=VENDOR_ID, idProduct=PRODUCT_ID)
        self.connect_usb()
        self.imagePath = None
        self.objects:LCDObject = []

        if isVertical:
            self.width,self.height=170,320
            self.imageBuffer = Image.effect_noise([self.width,self.height],0.5).convert("RGBA")
            self.imageBuffer = Image.new("RGBA",[self.width,self.height],(0,0,0,100))
            self.textBuffer = Image.new("RGBA", [self.width, self.height], (0, 0, 0, 100))
            self.d_width, self.d_height = d_width, d_height
        else:
            self.width,self.height=320,170
            self.imageBuffer = Image.effect_noise([self.width,self.height],0.5).convert("RGBA")
            self.imageBuffer = Image.new("RGBA",[self.width,self.height],(0,0,0,0))
            self.textBuffer = Image.new("RGBA", [self.width, self.height], (0, 0, 0, 0))
            self.d_width, self.d_height = d_height, d_width
        
        self.h_blocks = int(self.width / self.d_width)        
        self.v_blocks = int(self.height / self.d_height)
        print(f"INIT WIDTH {self.width} DWIDTH {self.d_width} HEIGHT {self.height} DHEIGHT {self.d_height}")
        self.isVertical:bool = isVertical

        self.orient()
        self.scheduler = sched.scheduler(time.time, time.sleep)
        self.dirty_rects = [[0 for x in range(self.h_blocks)] for y in range(self.v_blocks)] 
        self.mark_all_dirty()

        print(f"INIT IBUFF => {self.imageBuffer.size} TBUFF => {self.textBuffer.size} ")
        
    def connect_usb(self):
        """

        Raises:
            ValueError: [description]

        Returns:
            [type]: [description]
        """
        if self.device is None:
            raise ValueError('Device not found')
        # get an endpoint instance
        cfg = self.device.get_active_configuration()
        intf = cfg[(1, 0)]
        self.endpoint = usb.util.find_descriptor(
            intf,
            # match the first OUT endpoint
            custom_match=\
            lambda e: \
                usb.util.endpoint_direction(e.bEndpointAddress) == \
                usb.util.ENDPOINT_OUT)
        # print(self.endpoint)
        return (self.endpoint)

    def orient(self):
        """

        Args:
            orient (bool, optional): [description]. Defaults to True.
        """        
        command_h = [85, 161, 241, 1 , 0, 0, 0, 0 ]
        command_v = [85, 161, 241, 2 , 0, 0, 0, 0 ]
        if not self.isVertical: command_x = command_h 
        else: command_x = command_v
        buffer = [0] * 4096
        if self.isVertical : print("ORIENTATION VERTICAL")
        else : print("ORIENTATION HORIZONTAL")
        bcommand = bytearray (command_x)
        bbuffer = bytearray (buffer)
        final_ba = bcommand + bbuffer
        self.endpoint.write(final_ba)

    def black(self):
        """
        """
        print(f"Image size {self.imageBuffer.size}   Text Buffer { self.textBuffer.size }")
        self.imageBuffer =self.textBuffer = Image.new("RGBA",self.imageBuffer.size,(0,0,0,255))
        self.mark_all_dirty()
        self.render()

    def white(self):
        """
        """
        print(f"Image size {self.imageBuffer.size}   Text Buffer { self.textBuffer.size }")
        self.imageBuffer =self.textBuffer = Image.new("RGBA",self.imageBuffer.size,(255,255,255,255))
        self.mark_all_dirty()
        self.render()


    def rgb888_to_rgb565(self,r: int, g: int  , b:int):
        """
        Args:
            r (int): [description]
            g (int): [description]
            b (int): [description]

        Returns:
            [type]: [description]
        """
        rgb = ((r & 0b11111000) << 8) | ((g & 0b11111100) << 3) | (b >> 3);

        return rgb

    def convert_image_to_rgb565_part(self,image:Image):
        """
        Args:
            image (Image): [description]

        Returns:
            [type]: [description]
        """
        if image.format != "RGB": image = image.convert("RGB")
        height, width = image.size

        bx = bytearray(height * width * 2)
        i = 0 
        for y in range(0, width):
            for x in range(0, height): 
                r, g, b = image.getpixel((x, y))
                rgb565 = self.rgb888_to_rgb565(r, g, b).to_bytes(2, "big")
                bx[i] = rgb565[0]
                bx[i + 1] = rgb565[1]            
                i += 2
        return (bx)

    def part_updatei(self, image:Image, x:int, y:int, w:int, h:int):
        """
        Args:
            image (Image): [description]
            x (int): [description]
            y (int): [description]
            w (int): [description]
            h (int): [description]
        """
        start_time = time.time()
        if ((w * h) > 2080): return
        bcommand = bytearray()
        bcommand.append(0x55)
        bcommand.append(0xA2)
        bcommand[2:3] = (int(x).to_bytes(2, byteorder='little'))  # Offset
        bcommand[4:5] = (int(y).to_bytes(2, byteorder='little'))  # Length
        bcommand.append(w)
        bcommand.append(h)
        bbuffer = self.convert_image_to_rgb565_part(image)
        if (len(bbuffer) < 4096): 
            bbuffer += bytes(4096 - len(bbuffer))                
        final_ba = bcommand + bbuffer
        try:
            self.endpoint.write(final_ba)
        except:
            pass

    def print_dirty_set(self):
        """
        """
        pprint(self.dirty_rects)

    def mark_all_dirty(self):
        """
        """
        for i in range(self.h_blocks):
            for j in range(self.v_blocks):
                self.dirty_rects[j][i] += 1

    def mark_all_clean(self):
        """
        """
        for i in range(self.h_blocks):
            for j in range(self.v_blocks):
                self.dirty_rects[j][i] = 0

    def mark_dirty(self,bounds):
        x1,y1,x2,y2=bounds
        # If we are horizontal then the coordinates need   to be flipped

        # print(f"x1 y1  x2  y2 :: {bounds}")
        min_x = max(0, min(x1, x2))
        max_x = min(self.height - 1, max(x1, x2))
        min_y = max(0, min(y1, y2))
        max_y = min(self.width - 1 , max(y1, y2))

        if not self.isVertical:
            max_x = min(self.width - 1, max(x1, x2))
            max_y = min(self.height - 1 , max(y1, y2))
        else:
            max_x = min(self.width - 1, max(x1, x2))
            max_y = min(self.height  - 1 , max(y1, y2))

        # print(f" minX:{min_x} maxX:{max_x}  || minY:{min_y} maxY:{max_y} || W-{self.d_width} H-{self.d_height} ")

        if not self.isVertical:
            start_x = math.floor( min_x // self.d_width )
            end_x = math.ceil ( max_x // self.d_width )
            start_y = math.floor( min_y // self.d_height)
            end_y = math.ceil(max_y // self.d_height )
        else:
            start_x = math.floor( min_x // self.d_width )
            end_x = math.ceil ( max_x // self.d_width )
            start_y = math.floor( min_y // self.d_height)
            end_y = math.ceil(max_y // self.d_height )

        # print(f" startX:{start_x} endX:{end_x}  || startY:{start_y} endY:{end_y}")        

        if self.isVertical :
            for j in range(start_x, end_x + 1):
                for i in range(start_y, end_y + 1  ):
                    self.dirty_rects[i][self.h_blocks - j -1 ] += 1
        else :
            for j in range(start_x, end_x + 1):
                for i in range(start_y, end_y + 1 ):
                    # print(f"I={i} J={j}")
                    self.dirty_rects[i][j] += 1

    def load_image(self, imageName:str):
        """

        Args:
            imageName (str): [description]
        """
        print(f" Image name  : {(imageName)} Vertical:{self.isVertical} Width:{self.width} Height:{self.height}")
        image = Image.open(imageName)         
        print(f"Image size :: {image.size}")   
        if image.size == (self.width,self.height):            
            print("Image dimensions match")
            self.imageBuffer = image.convert("RGBA")            
        elif image.size == (self.height,self.width):            
            print("Image is rotated")
            self.imageBuffer = image.rotate(90,expand=True).convert("RGBA")            
        else:
            print("Image resized")
            self.imageBuffer = image.resize((self.width, self.height), Image.Resampling.LANCZOS).convert("RGBA")

        self.imageBuffer.save(f"{imageName}writtenX.png", "PNG")

        print(f" IBUFF => {self.imageBuffer.size} TBUFF => {self.textBuffer.size} ")        
        self.mark_all_dirty()


    def render_when_vertical(self, simulate:bool = False):
        r,g,b=random.randint(0,255),random.randint(0,255),random.randint(0,255)
        start_time = time.time_ns()
        for i in range(0,self.height,self.d_height):
            for j in range(0,self.width,self.d_width):
                x1,y1,x2,y2= i,j,i+self.d_height,j+self.d_width    
                x_index, y_index = int(i/self.d_height),int(j/self.d_width)
                if ( self.dirty_rects[x_index][y_index] > 0):
                    txtImage = (self.textBuffer.transpose(Image.Transpose.ROTATE_90)).crop([x1,y1,x2,y2])
                    tmpImage = (self.imageBuffer.transpose(Image.Transpose.ROTATE_90)).crop([x1,y1,x2,y2])
                    tmpImage = Image.alpha_composite(tmpImage,txtImage)
                    if simulate : tmpImage = Image.new("RGBA", tmpImage.size,(r,g,b))
                    self.part_updatei(tmpImage, i,j,self.d_height,self.d_width)
                    self.dirty_rects[x_index][y_index] -=1
        print(f"\t\t-> RWV time is {((time.time_ns() - start_time)/1000000):10.2f}ms")

    def render_when_horizontal(self, simulate:bool = False):
        r,g,b=random.randint(0,255),random.randint(0,255),random.randint(0,255)
        start_time = time.time_ns()
        for i in range(0,self.width,self.d_width):
            for j in range(0,self.height,self.d_height):    
                x1,y1,x2,y2= i,j,i+self.d_width,j+self.d_height
                x_index, y_index = int(j/self.d_height),int(i/self.d_width)                
                if ( self.dirty_rects[x_index][y_index] > 0):
                    txtImage = self.textBuffer.crop([x1,y1,x2,y2])   
                    tmpImage = self.imageBuffer.crop([x1,y1,x2,y2])   
                    tmpImage = Image.alpha_composite(txtImage,tmpImage)
                    if simulate : tmpImage = Image.new("RGBA", tmpImage.size,(r,g,b))
                    self.part_updatei(tmpImage, x1,y1,x2-x1,y2-y1)
                    self.dirty_rects[x_index][y_index] -=1
        print(f"\t\t-> RWH time is {((time.time_ns() - start_time)/1000000):10.2f}ms")

    def render(self, simulate:bool = False):
        if self.isVertical:
            self.render_when_vertical(simulate)
        else :
            self.render_when_horizontal(simulate)

    def drawObjects(self):
        for obj in self.objects:
            x,y = obj.x, obj.y
            self.textBuffer, bounds= obj.draw(self.imageBuffer, self.textBuffer)
            self.mark_dirty(bounds)  
        self.textBuffer.save("renTxtBuf.png")          

    def renderALL(self):
        """renderTime
        """
        print(f"Render Counter={self.timeCounter}")
        self.scheduler.enterabs(time.time() + 1, 1000, self.renderALL)        
        self.drawObjects()  
        self.render() 
        self.timeCounter+=1 

    def addObject(self, obj:LCDObject):
        self.objects.append(obj)

    def startScheduler(self):
        """startScheduler
        """
        self.scheduler.enterabs(time.time() + 1, 1000, self.renderALL)
        self.scheduler.run()        

def class_test(cIsVertical:bool):
    """
    """
    tft = S1TFT(34, 40, cIsVertical , "Megatron.otf")   
    tft.load_image("images/a4.jpg")
    tft.render()

    ob = LCDTime(10,0,fontName="CONTFU.ttf", fontSize=34,textColor=(255,255,0))
    tft.addObject(ob) 

    ob = LCDDate(0,40,fontName="CONTFU.ttf", fontSize=22)
    tft.addObject(ob) 

    ob = LCDCPUutil(10,240,fontName="CONTFU.ttf", fontSize=24)
    tft.addObject(ob) 

    ob = LCDCPUfreq(10,260,fontName="CONTFU.ttf", fontSize=26)
    tft.addObject(ob) 


    tft.startScheduler()


def main():
    """main
    """
    print("Main")
    class_test(True)

if __name__ == "__main__":
    """ Launcher
    """
    main()

